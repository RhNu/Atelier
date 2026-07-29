use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    DeletePromptChunkResult, DeletePromptPresetResult, PromptChunk, PromptChunkId, PromptPreset,
    PromptPresetId, PromptPresetKind, PromptResourceError, PromptResourceRepository,
    PromptResourceResult, UpsertPromptChunkRequest, UpsertPromptPresetRequest,
};

#[derive(Clone, Debug)]
pub struct PromptChunkService<R> {
    repository: R,
}

#[derive(Clone, Debug)]
pub struct PromptPresetService<R> {
    repository: R,
}

impl<R> PromptPresetService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> PromptChunkService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> PromptChunkService<R>
where
    R: PromptResourceRepository,
{
    /// Creates or updates a prompt chunk.
    ///
    /// # Errors
    /// Returns an error when the key conflicts, the target chunk is missing, or
    /// repository operations fail.
    pub async fn upsert_chunk(
        &self,
        request: UpsertPromptChunkRequest,
    ) -> PromptResourceResult<PromptChunk> {
        if let Some(existing) = self.repository.get_chunk_by_key(&request.key).await?
            && request
                .chunk_id
                .as_ref()
                .is_none_or(|id| id != &existing.id)
        {
            return Err(PromptResourceError::conflict(format!(
                "chunk key `{}` already exists",
                request.key.as_str()
            )));
        }

        let now = unix_ms();
        let existing = match &request.chunk_id {
            Some(id) => Some(self.require_chunk(id).await?),
            None => None,
        };
        let id = match (&request.chunk_id, &existing) {
            (Some(id), _) => id.clone(),
            (None, _) => self.repository.allocate_chunk_id().await?,
        };
        let old_key = existing.as_ref().map(|chunk| chunk.key.clone());
        let created_at_ms = existing.as_ref().map_or(now, |chunk| chunk.created_at_ms);
        let chunk = PromptChunk {
            id: id.clone(),
            key: request.key,
            content: request.content,
            category: request.category,
            description: request.description,
            preview_thumb: request.preview_thumb,
            created_at_ms,
            updated_at_ms: now,
        };
        if let Some(old_key) = old_key
            && old_key != chunk.key
        {
            self.repository
                .save_chunk_and_rewrite_references(chunk.clone(), &old_key)
                .await?;
        } else {
            self.repository.save_chunk(chunk.clone()).await?;
        }
        Ok(chunk)
    }

    /// Returns a chunk by id.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn get_chunk_by_id(
        &self,
        id: &PromptChunkId,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        self.repository.get_chunk_by_id(id).await
    }

    /// Returns a chunk by key.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn get_chunk_by_key(
        &self,
        key: &crate::PromptChunkKey,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        self.repository.get_chunk_by_key(key).await
    }

    /// Lists all chunks in key order.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn list_chunks(&self) -> PromptResourceResult<Vec<PromptChunk>> {
        let mut chunks = self.repository.list_chunks().await?;
        chunks.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(chunks)
    }

    /// Deletes an unreferenced chunk.
    ///
    /// # Errors
    /// Returns an error when other chunks still reference the target chunk.
    pub async fn delete_chunk(
        &self,
        id: &PromptChunkId,
    ) -> PromptResourceResult<DeletePromptChunkResult> {
        let Some(chunk) = self.repository.get_chunk_by_id(id).await? else {
            return Ok(DeletePromptChunkResult { deleted: false });
        };
        let references = self.repository.list_chunk_references(&chunk.key).await?;
        if !references.is_empty() {
            return Err(PromptResourceError::conflict("chunk is still referenced")
                .with_references(references.into_iter().map(|item| item.key).collect()));
        }
        self.repository.delete_chunk(id).await?;
        Ok(DeletePromptChunkResult { deleted: true })
    }

    async fn require_chunk(&self, id: &PromptChunkId) -> PromptResourceResult<PromptChunk> {
        self.repository
            .get_chunk_by_id(id)
            .await?
            .ok_or_else(|| PromptResourceError::not_found("chunk does not exist"))
    }
}

impl<R> PromptPresetService<R>
where
    R: PromptResourceRepository,
{
    /// Creates or updates a prompt preset.
    ///
    /// # Errors
    /// Returns an error when the request is invalid, target preset is missing,
    /// or repository operations fail.
    pub async fn upsert_preset(
        &self,
        request: UpsertPromptPresetRequest,
    ) -> PromptResourceResult<PromptPreset> {
        validate_preset_request(&request)?;

        let now = unix_ms();
        let existing = match &request.preset_id {
            Some(id) => Some(self.require_preset(id).await?),
            None => None,
        };
        let id = match (&request.preset_id, &existing) {
            (Some(id), _) => id.clone(),
            (None, _) => self.repository.allocate_preset_id().await?,
        };
        let created_at_ms = existing.as_ref().map_or(now, |preset| preset.created_at_ms);
        let preset = PromptPreset {
            id,
            kind: request.kind,
            name: normalize_required_name(&request.name)?,
            category: normalize_optional_text(request.category),
            description: normalize_optional_text(request.description),
            order: request.order,
            prompt_behavior: request.prompt_behavior,
            uc_behavior: request.uc_behavior,
            quality_override: normalize_optional_text(request.quality_override),
            uc_preset_override: normalize_optional_text(request.uc_preset_override),
            preview_thumb: request.preview_thumb,
            created_at_ms,
            updated_at_ms: now,
        };
        self.repository.save_preset(preset.clone()).await?;
        Ok(preset)
    }

    /// Returns a preset by id.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn get_preset_by_id(
        &self,
        id: &PromptPresetId,
    ) -> PromptResourceResult<Option<PromptPreset>> {
        self.repository.get_preset_by_id(id).await
    }

    /// Lists presets sorted by order, name, then id.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn list_presets(
        &self,
        kind: Option<PromptPresetKind>,
    ) -> PromptResourceResult<Vec<PromptPreset>> {
        let mut presets = self.repository.list_presets(kind).await?;
        presets.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(presets)
    }

    /// Deletes a prompt preset.
    ///
    /// # Errors
    /// Returns an error when repository operations fail.
    pub async fn delete_preset(
        &self,
        id: &PromptPresetId,
    ) -> PromptResourceResult<DeletePromptPresetResult> {
        if self.repository.get_preset_by_id(id).await?.is_none() {
            return Ok(DeletePromptPresetResult { deleted: false });
        }
        self.repository.delete_preset(id).await?;
        Ok(DeletePromptPresetResult { deleted: true })
    }

    async fn require_preset(&self, id: &PromptPresetId) -> PromptResourceResult<PromptPreset> {
        self.repository
            .get_preset_by_id(id)
            .await?
            .ok_or_else(|| PromptResourceError::not_found("preset does not exist"))
    }
}

fn validate_preset_request(request: &UpsertPromptPresetRequest) -> PromptResourceResult<()> {
    normalize_required_name(&request.name)?;
    if request.kind == PromptPresetKind::Character
        && (request
            .quality_override
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
            || request
                .uc_preset_override
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()))
    {
        return Err(PromptResourceError::invalid_request(
            "character presets cannot define generation overrides",
        ));
    }
    Ok(())
}

fn normalize_required_name(value: &str) -> PromptResourceResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PromptResourceError::invalid_request(
            "preset name cannot be empty",
        ));
    }
    Ok(trimmed.to_owned())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn unix_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
