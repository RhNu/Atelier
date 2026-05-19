use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_executor::block_on;
use nai_atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactSource, VisualAssetRef,
    VisualAssetRole,
};
use nai_atelier_gallery::{
    GalleryError, GalleryErrorKind, GalleryImageReference, GalleryIndex, GalleryItem,
    GalleryItemId, GalleryQuery, GallerySafetyOverride, GalleryService, GallerySourceKind,
    ImageReferenceTarget,
};
use nai_atelier_resource_catalog::{ResourceId, ResourceRef, ResourceVariantKind, VariantId};
use nai_atelier_safety::{ImageSafetyScore, SafetyAssessment};

#[test]
fn indexes_generated_director_and_imported_artifacts() {
    block_on(async {
        let index = FakeGalleryIndex::default();
        let service = GalleryService::new(index.clone());

        let generated = service
            .index_artifact(generated_artifact("generated-1", 30), 30, None)
            .await
            .unwrap();
        let director = service
            .index_artifact(director_artifact("director-1", 20), 20, None)
            .await
            .unwrap();
        let imported = service
            .index_artifact(imported_artifact("imported-1", 10), 10, None)
            .await
            .unwrap();

        assert_eq!(generated.artifact_kind, ArtifactKind::GeneratedImage);
        assert_eq!(generated.source_kind(), GallerySourceKind::Generation);
        assert_eq!(director.artifact_kind, ArtifactKind::DirectorResult);
        assert_eq!(director.source_kind(), GallerySourceKind::Director);
        assert_eq!(imported.artifact_kind, ArtifactKind::ImportedImage);
        assert_eq!(imported.source_kind(), GallerySourceKind::Import);
        assert_eq!(index.items().len(), 3);
    });
}

#[test]
fn reindexing_same_artifact_updates_deterministic_gallery_item() {
    block_on(async {
        let index = FakeGalleryIndex::default();
        let service = GalleryService::new(index.clone());

        let first = service
            .index_artifact(generated_artifact("artifact-1", 10), 10, None)
            .await
            .unwrap();
        let second = service
            .index_artifact(
                generated_artifact("artifact-1", 90),
                90,
                Some(SafetyAssessment::new(
                    ResourceRef::base(ResourceId::new("res-artifact-1")),
                    ImageSafetyScore::new(0.7).unwrap(),
                )),
            )
            .await
            .unwrap();

        assert_eq!(
            first.id,
            GalleryItemId::from_artifact_id(&ArtifactId::new("artifact-1"))
        );
        assert_eq!(second.id, first.id);
        assert_eq!(second.indexed_at_ms, 90);
        assert!(second.safety_assessment.is_some());
        assert_eq!(index.items().len(), 1);
    });
}

#[test]
fn reindexing_same_artifact_preserves_manual_safety_override() {
    block_on(async {
        let index = FakeGalleryIndex::default();
        let service = GalleryService::new(index.clone());
        let first = service
            .index_artifact(generated_artifact("artifact-1", 10), 10, None)
            .await
            .unwrap();
        service
            .set_safety_override(&first.id, Some(GallerySafetyOverride::Hidden))
            .await
            .unwrap();

        let second = service
            .index_artifact(
                generated_artifact("artifact-1", 90),
                90,
                Some(SafetyAssessment::new(
                    ResourceRef::base(ResourceId::new("res-artifact-1")),
                    ImageSafetyScore::new(0.9).unwrap(),
                )),
            )
            .await
            .unwrap();

        assert_eq!(second.id, first.id);
        assert_eq!(
            second.manual_safety_override,
            Some(GallerySafetyOverride::Hidden)
        );
        assert_eq!(second.indexed_at_ms, 90);
        assert!(second.safety_assessment.is_some());
        assert_eq!(index.items().len(), 1);
    });
}

#[test]
fn query_sorts_by_indexed_time_descending_and_paginates() {
    block_on(async {
        let service = populated_service().await;

        let page = service
            .query(GalleryQuery {
                offset: 1,
                limit: 2,
                ..GalleryQuery::default()
            })
            .await
            .unwrap();

        assert_eq!(
            item_ids(&page),
            vec![
                GalleryItemId::from_artifact_id(&ArtifactId::new("generated-2")),
                GalleryItemId::from_artifact_id(&ArtifactId::new("director-1")),
            ]
        );
    });
}

#[test]
fn query_filters_by_kind_source_and_manual_safety_override() {
    block_on(async {
        let index = FakeGalleryIndex::default();
        let service = GalleryService::new(index);
        let generated = service
            .index_artifact(generated_artifact("generated-1", 30), 30, None)
            .await
            .unwrap();
        let director = service
            .index_artifact(director_artifact("director-1", 20), 20, None)
            .await
            .unwrap();
        service
            .index_artifact(imported_artifact("imported-1", 10), 10, None)
            .await
            .unwrap();

        service
            .set_safety_override(&generated.id, Some(GallerySafetyOverride::Hidden))
            .await
            .unwrap();
        service
            .set_safety_override(&director.id, Some(GallerySafetyOverride::Sensitive))
            .await
            .unwrap();

        let generated_items = service
            .query(GalleryQuery {
                artifact_kind: Some(ArtifactKind::GeneratedImage),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();
        let director_items = service
            .query(GalleryQuery {
                source_kind: Some(GallerySourceKind::Director),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();
        let hidden_items = service
            .query(GalleryQuery {
                manual_safety_override: Some(GallerySafetyOverride::Hidden),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();

        assert_eq!(item_ids(&generated_items), vec![generated.id]);
        assert_eq!(item_ids(&director_items), vec![director.id]);
        assert_eq!(
            item_ids(&hidden_items),
            vec![GalleryItemId::from_artifact_id(&ArtifactId::new(
                "generated-1"
            ))]
        );
    });
}

#[test]
fn image_reference_preserves_target_for_each_supported_destination() {
    block_on(async {
        let index = FakeGalleryIndex::default();
        let service = GalleryService::new(index);
        let item = service
            .index_artifact(generated_artifact("artifact-1", 10), 10, None)
            .await
            .unwrap();

        for target in [
            ImageReferenceTarget::Director,
            ImageReferenceTarget::ImageToImage,
            ImageReferenceTarget::Vibe,
            ImageReferenceTarget::PreciseReference,
        ] {
            let reference = service.image_reference_for(&item.id, target).await.unwrap();

            assert_reference(
                &reference,
                &item,
                target,
                VisualAssetRole::Original,
                &ResourceRef::base(ResourceId::new("res-artifact-1")),
            );
        }
    });
}

#[test]
fn image_reference_prefers_original_then_sanitized_then_primary_resource() {
    block_on(async {
        let service = GalleryService::new(FakeGalleryIndex::default());

        let with_preview_and_sanitized = service
            .index_artifact(
                artifact_with_assets(
                    "sanitized-1",
                    vec![
                        asset(
                            VisualAssetRole::Preview,
                            ResourceRef::new(
                                ResourceId::new("res-sanitized-1"),
                                Some(VariantId::new("preview-1")),
                            ),
                            Some(ResourceVariantKind::Preview),
                        ),
                        asset(
                            VisualAssetRole::Sanitized,
                            ResourceRef::new(
                                ResourceId::new("res-sanitized-1"),
                                Some(VariantId::new("sanitized-1")),
                            ),
                            Some(ResourceVariantKind::Sanitized),
                        ),
                    ],
                ),
                10,
                None,
            )
            .await
            .unwrap();
        let sanitized_reference = service
            .image_reference_for(&with_preview_and_sanitized.id, ImageReferenceTarget::Vibe)
            .await
            .unwrap();
        assert_reference(
            &sanitized_reference,
            &with_preview_and_sanitized,
            ImageReferenceTarget::Vibe,
            VisualAssetRole::Sanitized,
            &ResourceRef::new(
                ResourceId::new("res-sanitized-1"),
                Some(VariantId::new("sanitized-1")),
            ),
        );

        let with_only_preview = service
            .index_artifact(
                artifact_with_assets(
                    "primary-1",
                    vec![asset(
                        VisualAssetRole::Preview,
                        ResourceRef::new(
                            ResourceId::new("res-primary-1"),
                            Some(VariantId::new("preview-1")),
                        ),
                        Some(ResourceVariantKind::Preview),
                    )],
                ),
                20,
                None,
            )
            .await
            .unwrap();
        let fallback_reference = service
            .image_reference_for(&with_only_preview.id, ImageReferenceTarget::Director)
            .await
            .unwrap();

        assert_reference(
            &fallback_reference,
            &with_only_preview,
            ImageReferenceTarget::Director,
            VisualAssetRole::Original,
            &ResourceRef::base(ResourceId::new("res-primary-1")),
        );
    });
}

#[test]
fn image_reference_for_missing_item_returns_not_found() {
    block_on(async {
        let service = GalleryService::new(FakeGalleryIndex::default());

        let error = service
            .image_reference_for(
                &GalleryItemId::new("artifact:missing"),
                ImageReferenceTarget::Director,
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind(), GalleryErrorKind::NotFound);
    });
}

#[derive(Clone, Default)]
struct FakeGalleryIndex {
    items: Arc<Mutex<BTreeMap<GalleryItemId, GalleryItem>>>,
}

impl FakeGalleryIndex {
    fn items(&self) -> Vec<GalleryItem> {
        self.items.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl GalleryIndex for FakeGalleryIndex {
    async fn upsert_item(&self, item: GalleryItem) -> nai_atelier_gallery::GalleryResult<()> {
        self.items.lock().unwrap().insert(item.id.clone(), item);
        Ok(())
    }

    async fn get_item(
        &self,
        id: &GalleryItemId,
    ) -> nai_atelier_gallery::GalleryResult<Option<GalleryItem>> {
        Ok(self.items.lock().unwrap().get(id).cloned())
    }

    async fn query_items(
        &self,
        query: GalleryQuery,
    ) -> nai_atelier_gallery::GalleryResult<Vec<GalleryItem>> {
        Ok(query.apply(self.items()))
    }

    async fn set_safety_override(
        &self,
        id: &GalleryItemId,
        manual_safety_override: Option<GallerySafetyOverride>,
    ) -> nai_atelier_gallery::GalleryResult<GalleryItem> {
        let mut item = self
            .items
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| GalleryError::not_found("gallery item does not exist"))?;
        item.manual_safety_override = manual_safety_override;
        self.items.lock().unwrap().insert(id.clone(), item.clone());
        Ok(item)
    }
}

async fn populated_service() -> GalleryService<FakeGalleryIndex> {
    let index = FakeGalleryIndex::default();
    let service = GalleryService::new(index);
    service
        .index_artifact(generated_artifact("generated-1", 40), 40, None)
        .await
        .unwrap();
    service
        .index_artifact(generated_artifact("generated-2", 30), 30, None)
        .await
        .unwrap();
    service
        .index_artifact(director_artifact("director-1", 20), 20, None)
        .await
        .unwrap();
    service
        .index_artifact(imported_artifact("imported-1", 10), 10, None)
        .await
        .unwrap();
    service
}

fn assert_reference(
    reference: &GalleryImageReference,
    item: &GalleryItem,
    target: ImageReferenceTarget,
    role: VisualAssetRole,
    resource: &ResourceRef,
) {
    assert_eq!(reference.item_id, item.id);
    assert_eq!(reference.artifact_id, item.artifact_id);
    assert_eq!(reference.target, target);
    assert_eq!(reference.asset.role, role);
    assert_eq!(&reference.resource, resource);
}

fn item_ids(items: &[GalleryItem]) -> Vec<GalleryItemId> {
    items.iter().map(|item| item.id.clone()).collect()
}

fn generated_artifact(id: &str, indexed_at_ms: u64) -> ArtifactRecord {
    artifact(
        id,
        ArtifactKind::GeneratedImage,
        ArtifactSource::GenerationJob {
            job_id: format!("job-{id}"),
            batch_id: Some("batch-1".to_owned()),
        },
        indexed_at_ms,
    )
}

fn director_artifact(id: &str, indexed_at_ms: u64) -> ArtifactRecord {
    artifact(
        id,
        ArtifactKind::DirectorResult,
        ArtifactSource::DirectorRun {
            run_id: format!("run-{id}"),
        },
        indexed_at_ms,
    )
}

fn imported_artifact(id: &str, indexed_at_ms: u64) -> ArtifactRecord {
    artifact(
        id,
        ArtifactKind::ImportedImage,
        ArtifactSource::Import {
            import_id: format!("import-{id}"),
        },
        indexed_at_ms,
    )
}

fn artifact(
    id: &str,
    kind: ArtifactKind,
    source: ArtifactSource,
    indexed_at_ms: u64,
) -> ArtifactRecord {
    let resource_id = ResourceId::new(format!("res-{id}"));
    ArtifactRecord {
        id: ArtifactId::new(id),
        kind,
        source,
        primary_resource: ResourceRef::base(resource_id.clone()),
        metadata: ArtifactMetadata {
            seed: Some(i64::try_from(indexed_at_ms).expect("test timestamp should fit i64")),
            model_name: Some("nai-diffusion-4-5-full".to_owned()),
            ..ArtifactMetadata::default()
        },
        replay: None,
        assets: vec![asset(
            VisualAssetRole::Original,
            ResourceRef::base(resource_id),
            Some(ResourceVariantKind::Original),
        )],
    }
}

fn artifact_with_assets(id: &str, assets: Vec<VisualAssetRef>) -> ArtifactRecord {
    ArtifactRecord {
        id: ArtifactId::new(id),
        kind: ArtifactKind::GeneratedImage,
        source: ArtifactSource::GenerationJob {
            job_id: format!("job-{id}"),
            batch_id: None,
        },
        primary_resource: ResourceRef::base(ResourceId::new(format!("res-{id}"))),
        metadata: ArtifactMetadata::default(),
        replay: None,
        assets,
    }
}

const fn asset(
    role: VisualAssetRole,
    resource: ResourceRef,
    variant_kind: Option<ResourceVariantKind>,
) -> VisualAssetRef {
    VisualAssetRef {
        role,
        resource,
        variant_kind,
    }
}
