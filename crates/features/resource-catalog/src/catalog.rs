use crate::model::{
    BlobId, CreateVariantRequest, RegisterResourceRequest, ReleaseOutcome, RepairReport,
    ResourceCleanupReport, ResourceLink, ResourceRecord, ResourceRef, ResourceState,
    ResourceVariant,
};
use crate::ports::{
    BuildVariantRequest, ResourceBlobStore, ResourceCatalogRepository, ResourceVariantBuilder,
};
use crate::{ResourceCatalogError, ResourceId, ResourceOwner, ResourceRelation, ResourceResult};

#[derive(Clone, Debug)]
pub struct ResourceCatalog<R, B, V> {
    repository: R,
    blob_store: B,
    variant_builder: V,
}

impl<R, B, V> ResourceCatalog<R, B, V> {
    #[must_use]
    pub const fn new(repository: R, blob_store: B, variant_builder: V) -> Self {
        Self {
            repository,
            blob_store,
            variant_builder,
        }
    }
}

impl<R, B, V> ResourceCatalog<R, B, V>
where
    R: ResourceCatalogRepository,
    B: ResourceBlobStore,
    V: ResourceVariantBuilder,
{
    /// Registers a resource through staged blob storage and a catalog transaction.
    ///
    /// # Errors
    /// Returns an error when staging/finalizing the blob fails, when catalog
    /// persistence fails, or when the final transaction cannot commit.
    pub async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        let staged = self.blob_store.stage_blob(request.blob).await?;
        let mut tx = match self.repository.begin_transaction().await {
            Ok(tx) => tx,
            Err(error) => {
                let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
                return Err(error);
            }
        };
        let resource_id = request.resource_id;
        let blob_id = staged.blob_id.clone();
        let link = ResourceLink::new(resource_id.clone(), request.owner, request.relation);
        let record = ResourceRecord {
            id: resource_id.clone(),
            kind: request.kind,
            lifecycle: request.lifecycle,
            state: ResourceState::Pending,
            blob_id: staged.blob_id,
            metadata: staged.metadata,
        };

        if let Err(error) = tx.insert_pending_record(record).await {
            let _rollback_result = tx.rollback().await;
            let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
            return Err(error);
        }
        if let Err(error) = tx.attach_owner(link).await {
            let _rollback_result = tx.rollback().await;
            let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
            return Err(error);
        }
        if let Err(error) = self.blob_store.finalize_blob(&staged.token).await {
            let _rollback_result = tx.rollback().await;
            let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
            return Err(error);
        }
        if let Err(error) = tx.mark_ready(&resource_id).await {
            let _rollback_result = tx.rollback().await;
            return Err(self.record_orphan_blob(&blob_id, error).await);
        }
        if let Err(error) = tx.commit().await {
            return Err(self.record_orphan_blob(&blob_id, error).await);
        }
        Ok(ResourceRef::base(resource_id))
    }

    /// Adds another owner link to a ready resource.
    ///
    /// # Errors
    /// Returns an error when the resource is not ready or catalog persistence
    /// fails.
    pub async fn attach_owner(
        &self,
        resource_id: &ResourceId,
        owner: ResourceOwner,
        relation: ResourceRelation,
    ) -> ResourceResult<()> {
        self.ready_record(resource_id).await?;
        let mut tx = self.repository.begin_transaction().await?;
        tx.attach_owner(ResourceLink::new(resource_id.clone(), owner, relation))
            .await?;
        tx.commit().await
    }

    /// Removes an owner link and marks auto-releasable resources for cleanup.
    ///
    /// # Errors
    /// Returns an error when the resource is not ready or catalog persistence
    /// fails.
    pub async fn detach_owner(
        &self,
        resource_id: &ResourceId,
        owner: &ResourceOwner,
        relation: ResourceRelation,
    ) -> ResourceResult<ReleaseOutcome> {
        let record = self.ready_record(resource_id).await?;
        let link = ResourceLink::new(resource_id.clone(), owner.clone(), relation);
        let mut tx = self.repository.begin_transaction().await?;
        tx.detach_owner(&link).await?;
        let remaining_owner_links = tx.count_owner_links(resource_id).await?;
        let delete_pending = remaining_owner_links == 0 && record.lifecycle.is_auto_releasable();
        if delete_pending {
            tx.mark_delete_pending(resource_id).await?;
        }
        tx.commit().await?;
        Ok(ReleaseOutcome {
            remaining_owner_links,
            delete_pending,
        })
    }

    /// Marks a ready resource as delete-pending when it has no owner links.
    ///
    /// This is an explicit cleanup operation for legacy records whose lifecycle
    /// was not auto-releasable when created.
    ///
    /// # Errors
    /// Returns an error when the resource is not ready or catalog persistence
    /// fails.
    pub async fn mark_delete_pending_if_unowned(
        &self,
        resource_id: &ResourceId,
    ) -> ResourceResult<bool> {
        self.ready_record(resource_id).await?;
        let mut tx = self.repository.begin_transaction().await?;
        let remaining_owner_links = tx.count_owner_links(resource_id).await?;
        let marked = remaining_owner_links == 0;
        if marked {
            tx.mark_delete_pending(resource_id).await?;
        }
        tx.commit().await?;
        Ok(marked)
    }

    /// Returns a ready resource record for an opaque reference.
    ///
    /// # Errors
    /// Returns an error when the resource is not ready, does not exist, or the
    /// repository cannot be queried.
    pub async fn get(&self, reference: &ResourceRef) -> ResourceResult<ResourceRecord> {
        if reference.variant_id.is_some() {
            return Err(ResourceCatalogError::invalid_state(
                "variant references must be resolved with get_variant",
            ));
        }
        self.ready_record(&reference.id).await
    }

    /// Lists ready resource references attached to an owner.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn list_by_owner(&self, owner: &ResourceOwner) -> ResourceResult<Vec<ResourceRef>> {
        self.repository.list_ready_refs_by_owner(owner).await
    }

    /// Lists ready owner links with their relation metadata.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be queried.
    pub async fn list_links_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceLink>> {
        self.repository.list_ready_links_by_owner(owner).await
    }

    /// Returns a registered resource variant.
    ///
    /// # Errors
    /// Returns an error when the variant does not exist or the repository
    /// cannot be queried.
    pub async fn get_variant(&self, id: &crate::VariantId) -> ResourceResult<ResourceVariant> {
        self.repository
            .get_variant(id)
            .await?
            .ok_or_else(|| ResourceCatalogError::not_found("resource variant does not exist"))
    }

    /// Creates a derived variant for a ready resource.
    ///
    /// # Errors
    /// Returns an error when the source resource is not ready, variant building
    /// fails, or catalog persistence fails.
    pub async fn create_variant(
        &self,
        request: CreateVariantRequest,
    ) -> ResourceResult<ResourceVariant> {
        let source_record = self.ready_record(&request.source.id).await?;
        let built = self
            .variant_builder
            .build_variant(BuildVariantRequest {
                source: request.source.clone(),
                source_record,
                variant_id: request.variant_id.clone(),
                kind: request.kind,
            })
            .await?;
        self.create_built_variant(request, built).await
    }

    /// Stores a caller-built derived variant for a ready resource.
    ///
    /// # Errors
    /// Returns an error when the source resource is not ready, blob staging
    /// fails, or catalog persistence fails.
    pub async fn create_built_variant(
        &self,
        request: CreateVariantRequest,
        built: crate::BuiltResourceVariant,
    ) -> ResourceResult<ResourceVariant> {
        self.ready_record(&request.source.id).await?;
        let staged = self.blob_store.stage_blob(built.blob).await?;
        let variant = ResourceVariant {
            id: request.variant_id,
            resource_id: request.source.id,
            kind: request.kind,
            blob_id: staged.blob_id.clone(),
            metadata: staged.metadata.clone(),
        };
        let mut tx = match self.repository.begin_transaction().await {
            Ok(tx) => tx,
            Err(error) => {
                let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
                return Err(error);
            }
        };
        if let Err(error) = tx.insert_variant(variant.clone()).await {
            let _rollback_result = tx.rollback().await;
            let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
            return Err(error);
        }
        if let Err(error) = self.blob_store.finalize_blob(&staged.token).await {
            let _rollback_result = tx.rollback().await;
            let _abort_result = self.blob_store.abort_staged_blob(&staged.token).await;
            return Err(error);
        }
        if let Err(error) = tx.commit().await {
            return Err(self.record_orphan_blob(&variant.blob_id, error).await);
        }
        Ok(variant)
    }

    /// Removes orphan blob markers and deletes their associated blobs.
    ///
    /// # Errors
    /// Returns an error when orphan scanning, blob deletion, or catalog marker
    /// cleanup fails.
    pub async fn repair_orphans(&self) -> ResourceResult<RepairReport> {
        let orphan_blobs = self.repository.scan_orphan_blobs().await?;
        let mut report = RepairReport::default();
        for blob_id in orphan_blobs {
            if self.blob_store.blob_exists(&blob_id).await? {
                self.blob_store.delete_blob(&blob_id).await?;
                report.deleted_orphan_blobs += 1;
            }
            let mut tx = self.repository.begin_transaction().await?;
            tx.clear_orphan_blob_marker(&blob_id).await?;
            tx.commit().await?;
            report.cleared_orphan_markers += 1;
        }
        Ok(report)
    }

    /// Deletes resources already marked `DeletePending` and removes unshared
    /// base and variant blobs from storage.
    ///
    /// # Errors
    /// Returns an error when catalog scanning, blob deletion, or catalog row
    /// cleanup fails. If blob deletion fails, the delete-pending catalog record
    /// is left in place so a later cleanup can retry it.
    pub async fn cleanup_delete_pending(&self) -> ResourceResult<ResourceCleanupReport> {
        let candidates = self.repository.list_delete_pending_resources().await?;
        let mut report = ResourceCleanupReport::default();
        for candidate in candidates {
            let mut deleted_blobs = 0;
            for blob_id in unique_blob_ids(&candidate.record.blob_id, &candidate.variants) {
                if self
                    .repository
                    .blob_is_referenced_outside_resource(&candidate.record.id, &blob_id)
                    .await?
                {
                    continue;
                }
                if self.blob_store.blob_exists(&blob_id).await? {
                    self.blob_store.delete_blob(&blob_id).await?;
                    deleted_blobs += 1;
                }
            }
            self.repository
                .delete_resource_record(&candidate.record.id)
                .await?;
            report.resources_deleted += 1;
            report.blobs_deleted += deleted_blobs;
        }
        Ok(report)
    }

    async fn ready_record(&self, id: &ResourceId) -> ResourceResult<ResourceRecord> {
        self.repository.get_ready_record(id).await?.ok_or_else(|| {
            ResourceCatalogError::not_found("resource is not ready or does not exist")
        })
    }

    async fn record_orphan_blob(
        &self,
        blob_id: &crate::BlobId,
        original_error: ResourceCatalogError,
    ) -> ResourceCatalogError {
        match self.repository.record_orphan_blob(blob_id).await {
            Ok(()) => original_error,
            Err(orphan_error) => ResourceCatalogError::repository(format!(
                "{original_error}; also failed to record orphan blob: {orphan_error}"
            )),
        }
    }
}

fn unique_blob_ids(base_blob_id: &BlobId, variants: &[ResourceVariant]) -> Vec<BlobId> {
    let mut blob_ids = vec![base_blob_id.clone()];
    for variant in variants {
        if !blob_ids.contains(&variant.blob_id) {
            blob_ids.push(variant.blob_id.clone());
        }
    }
    blob_ids
}
