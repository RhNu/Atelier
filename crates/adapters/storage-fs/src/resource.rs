use super::{
    BlobId, BlobWriteIntent, Engine, OpenOptions, PathBuf, ResourceBlobStore, ResourceCatalogError,
    ResourceCatalogRepository, ResourceKind, ResourceMetadata, ResourceRef, ResourceResult,
    STANDARD, StagedBlob, StagedBlobSidecar, StagedBlobToken, WorkspaceLayout, WorkspaceRoot,
    WorkspaceSlot, async_trait, fs, io, parse_sha256_blob_id, read_sidecar, remove_file_if_exists,
    resource_fs_error, sha256_hex, storage_path_for, unique_staged_blob_token,
    validate_staged_part_matches_sidecar, validate_token, write_json_resource_exclusive,
    write_staging_part,
};

#[derive(Clone, Debug)]
pub struct FileSystemResourceBlobStore {
    root: WorkspaceRoot,
}

impl FileSystemResourceBlobStore {
    #[must_use]
    pub const fn new(root: WorkspaceRoot, _layout: WorkspaceLayout) -> Self {
        Self { root }
    }

    /// Returns the final filesystem path for a catalog blob id.
    ///
    /// # Errors
    /// Returns an error when the blob id is not a storage-fs sha256 blob id.
    pub fn blob_path(&self, blob_id: &BlobId) -> ResourceResult<PathBuf> {
        let hash = parse_sha256_blob_id(blob_id)?;
        let prefix = hash
            .get(..2)
            .ok_or_else(|| ResourceCatalogError::blob_store("sha256 hash is too short"))?;
        Ok(self
            .root
            .join_relative(&storage_path_for(WorkspaceSlot::ResourceBlobs))
            .join("sha256")
            .join(prefix)
            .join(hash))
    }

    fn staging_part_path(&self, token: &StagedBlobToken) -> ResourceResult<PathBuf> {
        self.staging_path(token, "part")
    }

    fn staging_sidecar_path(&self, token: &StagedBlobToken) -> ResourceResult<PathBuf> {
        self.staging_path(token, "json")
    }

    fn staging_path(&self, token: &StagedBlobToken, extension: &str) -> ResourceResult<PathBuf> {
        validate_token(token)?;
        Ok(self
            .root
            .join_relative(&storage_path_for(WorkspaceSlot::ResourceStaging))
            .join(format!("{}.{}", token.as_str(), extension)))
    }
}

#[derive(Clone, Debug)]
pub struct FileSystemResourceContentReader<R> {
    repository: R,
    blob_store: FileSystemResourceBlobStore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceContent {
    pub kind: ResourceKind,
    pub bytes: Vec<u8>,
}

impl<R> FileSystemResourceContentReader<R> {
    #[must_use]
    pub const fn new(repository: R, blob_store: FileSystemResourceBlobStore) -> Self {
        Self {
            repository,
            blob_store,
        }
    }
}

impl<R> FileSystemResourceContentReader<R>
where
    R: ResourceCatalogRepository,
{
    /// Reads the byte payload for a ready catalog resource or one of its
    /// variants.
    ///
    /// # Errors
    /// Returns an error when the resource cannot be resolved or blob I/O fails.
    pub async fn read_resource_bytes(
        &self,
        reference: &ResourceRef,
    ) -> ResourceResult<ResourceContent> {
        let record = self
            .repository
            .get_ready_record(&reference.id)
            .await?
            .ok_or_else(|| ResourceCatalogError::not_found("resource does not exist"))?;
        let blob_id = if let Some(variant_id) = &reference.variant_id {
            let variant = self
                .repository
                .get_variant(variant_id)
                .await?
                .ok_or_else(|| {
                    ResourceCatalogError::not_found("resource variant does not exist")
                })?;
            if variant.resource_id != reference.id {
                return Err(ResourceCatalogError::invalid_state(
                    "resource variant belongs to another resource",
                ));
            }
            variant.blob_id
        } else {
            record.blob_id
        };
        let path = self.blob_store.blob_path(&blob_id)?;
        let bytes = fs::read(&path).map_err(|source| resource_fs_error(&path, source))?;
        Ok(ResourceContent {
            kind: record.kind,
            bytes,
        })
    }

    /// Reads a resource payload as UTF-8 text.
    ///
    /// # Errors
    /// Returns an error when bytes are not valid UTF-8.
    pub async fn read_resource_text(&self, reference: &ResourceRef) -> ResourceResult<String> {
        let content = self.read_resource_bytes(reference).await?;
        String::from_utf8(content.bytes)
            .map_err(|error| ResourceCatalogError::blob_store(error.to_string()))
    }

    /// Reads a resource payload and returns a base64 representation.
    ///
    /// # Errors
    /// Returns an error when the resource cannot be resolved or read.
    pub async fn read_resource_base64(&self, reference: &ResourceRef) -> ResourceResult<String> {
        self.read_resource_bytes(reference)
            .await
            .map(|content| STANDARD.encode(content.bytes))
    }
}

#[async_trait]
impl ResourceBlobStore for FileSystemResourceBlobStore {
    async fn stage_blob(&self, intent: BlobWriteIntent) -> ResourceResult<StagedBlob> {
        let BlobWriteIntent::Bytes(bytes) = intent;
        let hash = sha256_hex(&bytes);
        let blob_id = BlobId::new(format!("sha256:{hash}"));
        let metadata = ResourceMetadata {
            byte_size: Some(bytes.len() as u64),
            content_hash: Some(hash),
            ..ResourceMetadata::default()
        };
        let (token, part_path) = self.create_unique_staging_part(&bytes)?;
        let sidecar_path = self.staging_sidecar_path(&token)?;
        let sidecar = StagedBlobSidecar::from_blob(&blob_id, &metadata);
        if let Err(error) = write_json_resource_exclusive(&sidecar_path, &sidecar) {
            let _remove_result = fs::remove_file(&part_path);
            return Err(error);
        }
        Ok(StagedBlob {
            token,
            blob_id,
            metadata,
        })
    }

    async fn finalize_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        let part_path = self.staging_part_path(staged)?;
        let sidecar_path = self.staging_sidecar_path(staged)?;
        let sidecar = read_sidecar(&sidecar_path)?;
        let blob_id = BlobId::new(sidecar.blob_id.clone());
        let blob_path = self.blob_path(&blob_id)?;
        validate_staged_part_matches_sidecar(&part_path, &sidecar)?;
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).map_err(|source| resource_fs_error(parent, source))?;
        }
        if blob_path.exists() {
            fs::remove_file(&part_path).map_err(|source| resource_fs_error(&part_path, source))?;
        } else {
            fs::rename(&part_path, &blob_path)
                .map_err(|source| resource_fs_error(&blob_path, source))?;
        }
        remove_file_if_exists(&sidecar_path)?;
        Ok(())
    }

    async fn abort_staged_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        remove_file_if_exists(&self.staging_part_path(staged)?)?;
        remove_file_if_exists(&self.staging_sidecar_path(staged)?)?;
        Ok(())
    }

    async fn delete_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        remove_file_if_exists(&self.blob_path(blob_id)?)
    }

    async fn blob_exists(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        Ok(self.blob_path(blob_id)?.exists())
    }
}

impl FileSystemResourceBlobStore {
    fn create_unique_staging_part(
        &self,
        bytes: &[u8],
    ) -> ResourceResult<(StagedBlobToken, PathBuf)> {
        for _attempt in 0..32 {
            let token = unique_staged_blob_token();
            let part_path = self.staging_part_path(&token)?;
            if let Some(parent) = part_path.parent() {
                fs::create_dir_all(parent).map_err(|source| resource_fs_error(parent, source))?;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&part_path)
            {
                Ok(mut file) => {
                    if let Err(error) = write_staging_part(&mut file, &part_path, bytes) {
                        let _remove_result = fs::remove_file(&part_path);
                        return Err(error);
                    }
                    return Ok((token, part_path));
                }
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(resource_fs_error(&part_path, source)),
            }
        }
        Err(ResourceCatalogError::blob_store(
            "failed to create a unique staged blob token",
        ))
    }
}
