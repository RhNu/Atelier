use std::collections::{BTreeMap, BTreeSet};

use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{GenerationDraftDto, SaveGenerationDraftRequestDto};
use atelier_app_api::prompt::LexiconDraftTargetDto;
use atelier_generation::{
    GenerationDraftCharacterPositionMode, GenerationDraftPromptState, GenerationDraftSeedMode,
    GenerationDraftSnapshot, GenerationDraftVibe,
};
use atelier_prompt_lexicon::{ResolvedLexiconEntity, canonical_comparison_key};
use atelier_resource_catalog::{ResourceId, ResourceOwner, ResourceOwnerKind, ResourceRelation};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use super::generation::GenerationUseCases;
use crate::AppResult;
use crate::mapping::{generation_draft_to_domain, generation_draft_to_dto};

#[derive(Clone)]
struct DraftResourceLink {
    resource_id: ResourceId,
    relation: ResourceRelation,
}

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn get_draft(&self) -> AppResult<Option<GenerationDraftDto>> {
        let draft = self.app.inner.generation_drafts.load().await?;
        Ok(draft.as_ref().map(generation_draft_to_dto))
    }

    pub async fn save_draft(
        &self,
        request: SaveGenerationDraftRequestDto,
    ) -> AppResult<GenerationDraftDto> {
        let _write_guard = self.app.inner.generation_draft_write.lock().await;
        let draft = generation_draft_to_domain(request.draft);
        let previous = self.app.inner.generation_drafts.load().await?;
        let old_links = previous
            .as_ref()
            .map(draft_resource_links)
            .unwrap_or_default();
        let new_links = draft_resource_links(&draft);
        let owner = generation_draft_owner();
        let mut attached = Vec::new();

        {
            let catalog = &self.app.inner.resources;
            for (key, link) in &new_links {
                if !old_links.contains_key(key) {
                    catalog
                        .attach_owner(&link.resource_id, owner.clone(), link.relation)
                        .await?;
                    attached.push(link.clone());
                }
            }
        }

        let saved = match self.app.inner.generation_drafts.save(draft.clone()).await {
            Ok(value) => value,
            Err(error) => {
                let catalog = &self.app.inner.resources;
                for link in &attached {
                    let _ = catalog
                        .detach_owner(&link.resource_id, &owner, link.relation)
                        .await;
                }
                let _ = catalog.cleanup_delete_pending().await;
                return Err(error.into());
            }
        };

        {
            let catalog = &self.app.inner.resources;
            for (key, link) in &old_links {
                if !new_links.contains_key(key) {
                    catalog
                        .detach_owner(&link.resource_id, &owner, link.relation)
                        .await?;
                }
            }
            release_import_staging_links(catalog, &new_links).await?;
            catalog.cleanup_delete_pending().await?;
        }

        Ok(generation_draft_to_dto(&saved))
    }

    pub async fn clear_draft(&self) -> AppResult<()> {
        let _write_guard = self.app.inner.generation_draft_write.lock().await;
        self.app.inner.generation_drafts.clear().await?;
        let owner = generation_draft_owner();
        let catalog = &self.app.inner.resources;
        for link in catalog.list_links_by_owner(&owner).await? {
            catalog
                .detach_owner(&link.resource_id, &owner, link.relation)
                .await?;
        }
        catalog.cleanup_delete_pending().await?;
        Ok(())
    }

    pub async fn append_lexicon_entities(
        &self,
        target: LexiconDraftTargetDto,
        entities: &[ResolvedLexiconEntity],
    ) -> AppResult<GenerationDraftDto> {
        let _write_guard = self.app.inner.generation_draft_write.lock().await;
        let mut draft = if let Some(draft) = self.app.inner.generation_drafts.load().await? {
            draft
        } else {
            let settings = self.app.inner.settings.get_workspace_settings().await?;
            default_draft(&settings)
        };
        let current_model = draft.model;
        let state = draft
            .prompt_states
            .iter_mut()
            .find(|state| state.model == current_model)
            .ok_or_else(|| {
                crate::AppError::new(
                    "generation_draft_invalid_value",
                    "current model prompt state is missing",
                )
            })?;
        let prompt = match target {
            LexiconDraftTargetDto::Positive => &mut state.prompt,
            LexiconDraftTargetDto::Negative => &mut state.negative_prompt,
        };
        append_canonical_tags(prompt, entities);
        let saved = self.app.inner.generation_drafts.save(draft).await?;
        Ok(generation_draft_to_dto(&saved))
    }
}

fn default_draft(settings: &atelier_settings::WorkspaceSettings) -> GenerationDraftSnapshot {
    let defaults = &settings.generation;
    GenerationDraftSnapshot {
        model: defaults.model,
        prompt_states: vec![GenerationDraftPromptState {
            model: defaults.model,
            main_preset_id: None,
            prompt: String::new(),
            negative_prompt: String::new(),
            furry_mode: false,
            characters: Vec::new(),
            character_position_mode: GenerationDraftCharacterPositionMode::Global,
        }],
        size: defaults.size,
        quality: defaults.quality,
        transparent_background: defaults.transparent_background,
        uc_preset: defaults.uc_preset,
        steps: defaults.steps,
        scale: defaults.scale,
        sampler: defaults.sampler,
        noise_schedule: defaults.noise_schedule,
        seed_mode: if defaults.seed == 0 {
            GenerationDraftSeedMode::Random
        } else {
            GenerationDraftSeedMode::Fixed
        },
        seed: defaults.seed,
        n_samples: defaults.n_samples,
        request_count: 1,
        cfg_rescale: defaults.cfg_rescale,
        variety_boost: defaults.variety_boost,
        image_format: defaults.image_format,
        strict_mode: defaults.strict_mode,
        stream_enabled: true,
        i2i: None,
        vibe: GenerationDraftVibe {
            enabled: false,
            strength: 1.0,
            slots: Vec::new(),
        },
        precise_references: Vec::new(),
    }
}

fn append_canonical_tags(prompt: &mut String, entities: &[ResolvedLexiconEntity]) {
    let mut existing = prompt
        .split(',')
        .map(prompt_fragment_key)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    let mut additions = Vec::new();
    for entity in entities {
        let canonical = canonical_comparison_key(&entity.canonical_name);
        let already_present = existing.contains(&canonical)
            || entity
                .aliases
                .iter()
                .map(|alias| canonical_comparison_key(alias))
                .any(|alias| existing.contains(&alias));
        if already_present {
            continue;
        }
        existing.insert(canonical);
        additions.push(entity.canonical_name.as_str());
    }
    if additions.is_empty() {
        return;
    }
    if !prompt.trim().is_empty() {
        if prompt.trim_end().ends_with(',') {
            if prompt.trim_end().len() == prompt.len() {
                prompt.push(' ');
            }
        } else {
            prompt.push_str(", ");
        }
    }
    prompt.push_str(&additions.join(", "));
    prompt.push_str(", ");
}

fn prompt_fragment_key(fragment: &str) -> String {
    let mut value = fragment.trim();
    if let Some((prefix, remainder)) = value.split_once("::")
        && prefix.parse::<f32>().is_ok()
    {
        value = remainder.strip_suffix("::").unwrap_or(remainder);
    }
    value = value.trim_matches(|character| matches!(character, '{' | '}' | '[' | ']'));
    canonical_comparison_key(value)
}

async fn release_import_staging_links(
    catalog: &atelier_resource_catalog::ResourceCatalog<
        impl atelier_resource_catalog::ResourceCatalogRepository,
        impl atelier_resource_catalog::ResourceBlobStore,
        impl atelier_resource_catalog::ResourceVariantBuilder,
    >,
    new_links: &BTreeMap<String, DraftResourceLink>,
) -> AppResult<()> {
    let staging_owner = ResourceOwner::new(ResourceOwnerKind::ImportStaging, "user-image-inputs");
    let imported_resource_ids = new_links
        .values()
        .filter(|link| link.resource_id.as_str().starts_with("resource:import:"))
        .map(|link| link.resource_id.as_str())
        .collect::<BTreeSet<_>>();
    for staging_link in catalog.list_links_by_owner(&staging_owner).await? {
        if imported_resource_ids.contains(staging_link.resource_id.as_str()) {
            catalog
                .detach_owner(
                    &staging_link.resource_id,
                    &staging_owner,
                    staging_link.relation,
                )
                .await?;
        }
    }
    Ok(())
}

fn generation_draft_owner() -> ResourceOwner {
    ResourceOwner::new(ResourceOwnerKind::Workspace, "generation-draft")
}

fn draft_resource_links(draft: &GenerationDraftSnapshot) -> BTreeMap<String, DraftResourceLink> {
    let mut links = BTreeMap::new();
    if let Some(i2i) = &draft.i2i {
        insert_draft_link(&mut links, &i2i.image.id, ResourceRelation::Source);
        if let Some(inpaint) = &i2i.inpaint {
            insert_draft_link(
                &mut links,
                &inpaint.region_to_replace.id,
                ResourceRelation::Source,
            );
            for inset in &inpaint.reference_insets {
                insert_draft_link(&mut links, &inset.image.id, ResourceRelation::Reference);
            }
        }
    }
    for slot in &draft.vibe.slots {
        insert_draft_link(&mut links, &slot.encoding.id, ResourceRelation::Encoding);
        if let Some(source) = &slot.source_image {
            insert_draft_link(&mut links, &source.id, ResourceRelation::Source);
        }
    }
    for reference in &draft.precise_references {
        insert_draft_link(&mut links, &reference.image.id, ResourceRelation::Reference);
    }
    links
}

fn insert_draft_link(
    links: &mut BTreeMap<String, DraftResourceLink>,
    resource_id: &ResourceId,
    relation: ResourceRelation,
) {
    links.insert(
        format!("{}:{relation:?}", resource_id.as_str()),
        DraftResourceLink {
            resource_id: resource_id.clone(),
            relation,
        },
    );
}

#[cfg(test)]
mod tests {
    use atelier_prompt_lexicon::ResolvedLexiconEntity;

    use super::append_canonical_tags;

    #[test]
    fn append_preserves_prompt_syntax_and_deduplicates_aliases() {
        let mut prompt = "1.2::solo::, {{red hair}}, unknown_fragment".to_owned();
        append_canonical_tags(
            &mut prompt,
            &[
                entity(1, "red_hair", &["red hair"]),
                entity(2, "blue_hair", &["azure_hair"]),
            ],
        );

        assert_eq!(
            prompt,
            "1.2::solo::, {{red hair}}, unknown_fragment, blue_hair, "
        );
    }

    #[test]
    fn append_keeps_existing_trailing_separator_stable() {
        let mut prompt = "solo, ".to_owned();
        append_canonical_tags(&mut prompt, &[entity(1, "1girl", &[])]);
        assert_eq!(prompt, "solo, 1girl, ");
    }

    fn entity(id: u64, canonical: &str, aliases: &[&str]) -> ResolvedLexiconEntity {
        ResolvedLexiconEntity {
            entity_id: id,
            canonical_name: canonical.to_owned(),
            aliases: aliases.iter().map(|value| (*value).to_owned()).collect(),
        }
    }
}
