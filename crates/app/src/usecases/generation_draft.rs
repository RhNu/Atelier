use std::collections::{BTreeMap, BTreeSet};

use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{GenerationDraftDto, SaveGenerationDraftRequestDto};
use atelier_generation::GenerationDraftSnapshot;
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
            let kernel = self.app.inner.kernel.lock().await;
            let catalog = &kernel.ports().resources;
            for (key, link) in &new_links {
                if !old_links.contains_key(key) {
                    catalog
                        .attach_owner(&link.resource_id, owner.clone(), link.relation)
                        .await?;
                    attached.push(link.clone());
                }
            }
            drop(kernel);
        }

        let saved = match self.app.inner.generation_drafts.save(draft.clone()).await {
            Ok(value) => value,
            Err(error) => {
                let kernel = self.app.inner.kernel.lock().await;
                let catalog = &kernel.ports().resources;
                for link in &attached {
                    let _ = catalog
                        .detach_owner(&link.resource_id, &owner, link.relation)
                        .await;
                }
                let _ = catalog.cleanup_delete_pending().await;
                drop(kernel);
                return Err(error.into());
            }
        };

        {
            let kernel = self.app.inner.kernel.lock().await;
            let catalog = &kernel.ports().resources;
            for (key, link) in &old_links {
                if !new_links.contains_key(key) {
                    catalog
                        .detach_owner(&link.resource_id, &owner, link.relation)
                        .await?;
                }
            }
            release_import_staging_links(catalog, &new_links).await?;
            catalog.cleanup_delete_pending().await?;
            drop(kernel);
        }

        Ok(generation_draft_to_dto(&saved))
    }

    pub async fn clear_draft(&self) -> AppResult<()> {
        self.app.inner.generation_drafts.clear().await?;
        let owner = generation_draft_owner();
        let kernel = self.app.inner.kernel.lock().await;
        let catalog = &kernel.ports().resources;
        for link in catalog.list_links_by_owner(&owner).await? {
            catalog
                .detach_owner(&link.resource_id, &owner, link.relation)
                .await?;
        }
        catalog.cleanup_delete_pending().await?;
        drop(kernel);
        Ok(())
    }
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
        if let Some(mask) = &i2i.mask {
            insert_draft_link(&mut links, &mask.id, ResourceRelation::Source);
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
