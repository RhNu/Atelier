mod support;

use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, CreateVariantRequest, RegisterResourceRequest, RepairReport,
    ResourceCatalog, ResourceCatalogRepository, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRef, ResourceRelation, ResourceState,
    ResourceVariantBuilder, ResourceVariantKind, VariantId,
};
use futures_executor::block_on;

use support::{FakeBlobStore, FakeRepository, FakeVariantBuilder, assert_release_outcome};

#[test]
fn registers_resource_through_staged_blob_then_ready_record() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository.clone(), blob_store.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let reference = register_generated(&catalog, owner.clone(), ResourceId::new("res-1"))
            .await
            .unwrap();

        assert_eq!(reference, ResourceRef::base(ResourceId::new("res-1")));
        assert_eq!(
            blob_store.operations(),
            vec!["stage:3".to_owned(), "finalize:staged-1".to_owned()]
        );
        let record = catalog.get(&reference).await.unwrap();
        assert_eq!(record.state, ResourceState::Ready);
        assert_eq!(record.metadata.byte_size, Some(3));
        assert_eq!(
            catalog.list_by_owner(&owner).await.unwrap(),
            vec![reference]
        );
    });
}

#[test]
fn begin_transaction_failure_aborts_staged_blob() {
    block_on(async {
        let repository = FakeRepository::failing_begin();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository, blob_store.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let error = register_generated(&catalog, owner, ResourceId::new("res-1"))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("begin failed"));
        assert_eq!(
            blob_store.operations(),
            vec!["stage:3".to_owned(), "abort:staged-1".to_owned()]
        );
    });
}

#[test]
fn finalize_failure_rolls_back_and_aborts_staged_blob() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::failing_finalize();
        let catalog = test_catalog(repository.clone(), blob_store.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let error = register_generated(&catalog, owner, ResourceId::new("res-1"))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("finalize failed"));
        assert_eq!(
            blob_store.operations(),
            vec![
                "stage:3".to_owned(),
                "finalize:staged-1".to_owned(),
                "abort:staged-1".to_owned()
            ]
        );
        assert!(repository.records().is_empty());
    });
}

#[test]
fn mark_ready_failure_leaves_repairable_orphan_blob() {
    block_on(async {
        let repository = FakeRepository::failing_mark_ready();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository, blob_store.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let error = register_generated(&catalog, owner, ResourceId::new("res-1"))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("mark ready failed"));
        assert_eq!(blob_store.finalized_blobs(), vec![BlobId::new("blob-1")]);
        assert_eq!(
            catalog.repair_orphans().await.unwrap().deleted_orphan_blobs,
            1
        );
        assert!(blob_store.finalized_blobs().is_empty());
    });
}

#[test]
fn commit_failure_leaves_repairable_orphan_blob() {
    block_on(async {
        let repository = FakeRepository::failing_commit();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository, blob_store.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let error = register_generated(&catalog, owner, ResourceId::new("res-1"))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("commit failed"));
        assert_eq!(blob_store.finalized_blobs(), vec![BlobId::new("blob-1")]);
        assert_eq!(
            catalog.repair_orphans().await.unwrap(),
            RepairReport {
                deleted_orphan_blobs: 1,
                cleared_orphan_markers: 1,
            }
        );
        assert!(blob_store.finalized_blobs().is_empty());
    });
}

#[test]
fn multi_owner_links_release_only_the_detached_owner() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository, blob_store);
        let job_owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let gallery_owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, "gallery-1");
        let reference = register_cache_resource(&catalog, job_owner.clone()).await;

        catalog
            .attach_owner(
                &reference.id,
                gallery_owner.clone(),
                ResourceRelation::Reference,
            )
            .await
            .unwrap();

        let outcome = catalog
            .detach_owner(&reference.id, &job_owner, ResourceRelation::Primary)
            .await
            .unwrap();

        assert_release_outcome(outcome, 1, false);
        assert!(catalog.list_by_owner(&job_owner).await.unwrap().is_empty());
        assert_eq!(
            catalog.list_by_owner(&gallery_owner).await.unwrap(),
            vec![reference]
        );
    });
}

#[test]
fn list_links_by_owner_preserves_relation_metadata() {
    block_on(async {
        let catalog = test_catalog(FakeRepository::default(), FakeBlobStore::default());
        let owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, "gallery-1");
        let reference = register_cache_resource(&catalog, owner.clone()).await;

        let links = catalog.list_links_by_owner(&owner).await.unwrap();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].resource_id, reference.id);
        assert_eq!(links[0].relation, ResourceRelation::Primary);
    });
}

#[test]
fn last_cache_owner_marks_delete_pending_and_hides_from_queries() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository, blob_store);
        let owner = ResourceOwner::new(ResourceOwnerKind::Cache, "preview-cache");
        let reference = register_cache_resource(&catalog, owner.clone()).await;

        let outcome = catalog
            .detach_owner(&reference.id, &owner, ResourceRelation::Primary)
            .await
            .unwrap();

        assert_release_outcome(outcome, 0, true);
        assert!(catalog.get(&reference).await.is_err());
        assert!(catalog.list_by_owner(&owner).await.unwrap().is_empty());
    });
}

#[test]
fn workspace_scoped_resource_is_not_marked_delete_pending_on_last_release() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::default();
        let catalog = test_catalog(repository.clone(), blob_store);
        let owner = ResourceOwner::new(ResourceOwnerKind::Workspace, "default");
        let reference = catalog
            .register_resource(RegisterResourceRequest {
                resource_id: ResourceId::new("res-workspace"),
                kind: ResourceKind::SourceImage,
                lifecycle: ResourceLifecycle::WorkspaceScoped,
                owner: owner.clone(),
                relation: ResourceRelation::Primary,
                blob: BlobWriteIntent::Bytes(vec![9]),
            })
            .await
            .unwrap();

        let outcome = catalog
            .detach_owner(&reference.id, &owner, ResourceRelation::Primary)
            .await
            .unwrap();

        assert_release_outcome(outcome, 0, false);
        assert_eq!(
            repository.records()[&reference.id].state,
            ResourceState::Ready
        );
    });
}

#[test]
fn create_variant_stages_blob_and_persists_variant() {
    block_on(async {
        let repository = FakeRepository::default();
        let blob_store = FakeBlobStore::default();
        let variant_builder = FakeVariantBuilder::default();
        let catalog = ResourceCatalog::new(repository, blob_store.clone(), variant_builder.clone());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let source = register_cache_resource(&catalog, owner).await;

        let variant = create_preview_variant(&catalog, source.clone())
            .await
            .unwrap();

        assert_eq!(variant.resource_id, source.id);
        assert_eq!(variant.kind, ResourceVariantKind::Preview);
        assert_eq!(variant.metadata.byte_size, Some(7));
        assert_eq!(variant_builder.requests().len(), 1);
        assert_eq!(catalog.get_variant(&variant.id).await.unwrap(), variant);
        assert_eq!(
            blob_store.operations(),
            vec![
                "stage:1".to_owned(),
                "finalize:staged-1".to_owned(),
                "stage:7".to_owned(),
                "finalize:staged-2".to_owned(),
            ]
        );
    });
}

#[test]
fn variant_insert_failure_aborts_staged_variant_blob() {
    block_on(async {
        let repository = FakeRepository::failing_variant_insert();
        let blob_store = FakeBlobStore::default();
        let catalog = ResourceCatalog::new(
            repository,
            blob_store.clone(),
            FakeVariantBuilder::default(),
        );
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let source = register_cache_resource(&catalog, owner).await;

        let error = create_preview_variant(&catalog, source).await.unwrap_err();

        assert!(error.to_string().contains("insert variant failed"));
        assert!(
            blob_store
                .operations()
                .contains(&"abort:staged-2".to_owned())
        );
    });
}

#[test]
fn variant_commit_failure_leaves_repairable_orphan_blob() {
    block_on(async {
        let repository = FakeRepository::failing_commit_after_first_commit();
        let blob_store = FakeBlobStore::default();
        let catalog = ResourceCatalog::new(
            repository,
            blob_store.clone(),
            FakeVariantBuilder::default(),
        );
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let source = register_cache_resource(&catalog, owner).await;

        let error = create_preview_variant(&catalog, source).await.unwrap_err();

        assert!(error.to_string().contains("commit failed"));
        assert_eq!(
            blob_store.finalized_blobs(),
            vec![BlobId::new("blob-1"), BlobId::new("blob-2")]
        );
        assert_eq!(
            catalog.repair_orphans().await.unwrap().deleted_orphan_blobs,
            1
        );
        assert_eq!(blob_store.finalized_blobs(), vec![BlobId::new("blob-1")]);
    });
}

#[test]
fn get_rejects_variant_refs_instead_of_silently_returning_base_record() {
    block_on(async {
        let catalog = test_catalog(FakeRepository::default(), FakeBlobStore::default());
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let source = register_cache_resource(&catalog, owner).await;

        let error = catalog
            .get(&ResourceRef::new(
                source.id,
                Some(VariantId::new("preview-1")),
            ))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("variant references"));
    });
}

fn test_catalog(
    repository: FakeRepository,
    blob_store: FakeBlobStore,
) -> ResourceCatalog<FakeRepository, FakeBlobStore, FakeVariantBuilder> {
    ResourceCatalog::new(repository, blob_store, FakeVariantBuilder::default())
}

async fn register_generated<R, B, V>(
    catalog: &ResourceCatalog<R, B, V>,
    owner: ResourceOwner,
    resource_id: ResourceId,
) -> atelier_resource_catalog::ResourceResult<ResourceRef>
where
    R: ResourceCatalogRepository,
    B: atelier_resource_catalog::ResourceBlobStore,
    V: ResourceVariantBuilder,
{
    catalog
        .register_resource(RegisterResourceRequest {
            resource_id,
            kind: ResourceKind::GeneratedImage,
            lifecycle: ResourceLifecycle::JobScoped,
            owner,
            relation: ResourceRelation::Primary,
            blob: BlobWriteIntent::Bytes(vec![1, 2, 3]),
        })
        .await
}

async fn register_cache_resource<R, B, V>(
    catalog: &ResourceCatalog<R, B, V>,
    owner: ResourceOwner,
) -> ResourceRef
where
    R: ResourceCatalogRepository,
    B: atelier_resource_catalog::ResourceBlobStore,
    V: ResourceVariantBuilder,
{
    catalog
        .register_resource(RegisterResourceRequest {
            resource_id: ResourceId::new("res-cache"),
            kind: ResourceKind::PromptThumb,
            lifecycle: ResourceLifecycle::Cache,
            owner,
            relation: ResourceRelation::Primary,
            blob: BlobWriteIntent::Bytes(vec![1]),
        })
        .await
        .unwrap()
}

async fn create_preview_variant<R, B, V>(
    catalog: &ResourceCatalog<R, B, V>,
    source: ResourceRef,
) -> atelier_resource_catalog::ResourceResult<atelier_resource_catalog::ResourceVariant>
where
    R: ResourceCatalogRepository,
    B: atelier_resource_catalog::ResourceBlobStore,
    V: ResourceVariantBuilder,
{
    catalog
        .create_variant(CreateVariantRequest {
            source,
            variant_id: VariantId::new("preview-1"),
            kind: ResourceVariantKind::Preview,
        })
        .await
}
