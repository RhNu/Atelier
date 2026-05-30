use async_trait::async_trait;

use crate::{
    ChunkReference, PromptChunk, PromptChunkId, PromptChunkKey, PromptPreset, PromptPresetId,
    PromptPresetKind, PromptResourceResult,
};

#[async_trait]
pub trait PromptResourceReader: Send + Sync {
    async fn get_chunk_by_id(
        &self,
        id: &PromptChunkId,
    ) -> PromptResourceResult<Option<PromptChunk>>;

    async fn get_chunk_by_key(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Option<PromptChunk>>;

    async fn list_chunks(&self) -> PromptResourceResult<Vec<PromptChunk>>;

    async fn get_preset_by_id(
        &self,
        id: &PromptPresetId,
    ) -> PromptResourceResult<Option<PromptPreset>>;

    async fn list_presets(
        &self,
        kind: Option<PromptPresetKind>,
        include_disabled: bool,
    ) -> PromptResourceResult<Vec<PromptPreset>>;
}

#[async_trait]
pub trait PromptResourceRepository: PromptResourceReader {
    async fn allocate_chunk_id(&self) -> PromptResourceResult<PromptChunkId>;

    async fn save_chunk(&self, chunk: PromptChunk) -> PromptResourceResult<()>;

    async fn save_chunk_and_rewrite_references(
        &self,
        chunk: PromptChunk,
        old_key: &PromptChunkKey,
    ) -> PromptResourceResult<()>;

    async fn delete_chunk(&self, id: &PromptChunkId) -> PromptResourceResult<()>;

    async fn list_chunk_references(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Vec<ChunkReference>>;

    async fn allocate_preset_id(&self) -> PromptResourceResult<PromptPresetId>;

    async fn save_preset(&self, preset: PromptPreset) -> PromptResourceResult<()>;

    async fn delete_preset(&self, id: &PromptPresetId) -> PromptResourceResult<()>;
}
