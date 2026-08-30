use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use async_trait::async_trait;
use atelier_downloadable_resources::{
    DownloadableResourceCatalog, DownloadableResourceDescriptor, DownloadableResourceError,
    DownloadableResourceManager, DownloadableResourceResult, DownloadableResourceState,
    DownloadableResourceStatus, InstalledResource, ResourceInstallProgress,
    ResourceInstallProgressSink, validate_catalog,
};

use crate::catalog::CatalogDocument;
use crate::download::{download_file, verify};
use crate::state::{InstalledState, operation, write_json_atomic};

pub struct FileSystemDownloadableResourceManager {
    inner: Arc<ManagerInner>,
}

struct ManagerInner {
    root: PathBuf,
    catalog_url: String,
    seed_catalog: &'static str,
    client: reqwest::Client,
    catalog: Mutex<Option<DownloadableResourceCatalog>>,
    state: Mutex<InstalledState>,
    cancellations: Mutex<HashMap<String, Arc<AtomicBool>>>,
    activities: Mutex<HashMap<String, InstallActivity>>,
    failures: Mutex<HashMap<String, String>>,
    leases: Mutex<HashMap<String, Weak<ResourceLease>>>,
}

#[derive(Clone)]
struct InstallActivity {
    state: DownloadableResourceState,
    downloaded_bytes: u64,
}

struct ResourceLease {
    key: String,
    manager: Weak<ManagerInner>,
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        if let Some(manager) = self.manager.upgrade() {
            manager.finish_pending_delete(&self.key);
        }
    }
}

impl FileSystemDownloadableResourceManager {
    /// Creates a manager rooted at the application data directory.
    ///
    /// # Errors
    /// Returns an error when the root, state, or HTTP client cannot be initialized.
    pub fn new(
        root: impl Into<PathBuf>,
        catalog_url: impl Into<String>,
        seed_catalog: &'static str,
    ) -> DownloadableResourceResult<Arc<Self>> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(operation)?;
        let state = InstalledState::read(&root.join("state.json"))?;
        let client = reqwest::Client::builder()
            .user_agent("Atelier/0.5 downloadable-resources")
            .build()
            .map_err(operation)?;
        Ok(Arc::new(Self {
            inner: Arc::new(ManagerInner {
                root,
                catalog_url: catalog_url.into(),
                seed_catalog,
                client,
                catalog: Mutex::new(None),
                state: Mutex::new(state),
                cancellations: Mutex::new(HashMap::new()),
                activities: Mutex::new(HashMap::new()),
                failures: Mutex::new(HashMap::new()),
                leases: Mutex::new(HashMap::new()),
            }),
        }))
    }

    /// Renames the exact pre-0.5 image-analysis directory and removes it in the background once.
    ///
    /// # Errors
    /// Returns an error when path containment cannot be proven or the rename/marker write fails.
    pub fn cleanup_legacy_image_analysis(
        &self,
        app_data_dir: &Path,
    ) -> DownloadableResourceResult<()> {
        let mut state = self.inner.lock_state()?;
        if state.legacy_cleanup_complete {
            return Ok(());
        }
        let legacy = app_data_dir.join("models").join("image-analysis");
        if legacy.exists() {
            let canonical_app_data = fs::canonicalize(app_data_dir).map_err(operation)?;
            let parent = legacy.parent().ok_or_else(|| {
                DownloadableResourceError::Operation("legacy model path has no parent".to_owned())
            })?;
            let canonical_parent = fs::canonicalize(parent).map_err(operation)?;
            if !canonical_parent.starts_with(&canonical_app_data)
                || self.inner.root.starts_with(&legacy)
            {
                return Err(DownloadableResourceError::Operation(
                    "refusing to remove legacy resources outside app data".to_owned(),
                ));
            }
            let deleting = parent.join(format!(
                "image-analysis.deleting-0.5.0-{}",
                std::process::id()
            ));
            fs::rename(&legacy, &deleting).map_err(operation)?;
            let mut updated = state.clone();
            updated.legacy_cleanup_complete = true;
            if let Err(error) = updated.write(&self.inner.state_path()) {
                let _ = fs::rename(&deleting, &legacy);
                return Err(error);
            }
            *state = updated;
            drop(state);
            std::thread::spawn(move || {
                if let Err(error) = fs::remove_dir_all(&deleting) {
                    log::warn!("failed to remove renamed legacy model directory: {error}");
                }
            });
            return Ok(());
        }
        let mut updated = state.clone();
        updated.legacy_cleanup_complete = true;
        updated.write(&self.inner.state_path())?;
        *state = updated;
        drop(state);
        Ok(())
    }

    async fn load_catalog(
        &self,
        refresh: bool,
    ) -> DownloadableResourceResult<DownloadableResourceCatalog> {
        if !refresh && let Some(catalog) = self.inner.lock_catalog()?.clone() {
            return Ok(catalog);
        }
        let remote = if refresh || !self.inner.cache_path().is_file() {
            match self.fetch_catalog().await {
                Ok(bytes) => Some(bytes),
                Err(error) if self.inner.cache_path().is_file() => {
                    log::warn!("resource catalog refresh failed; using cached catalog: {error}");
                    None
                }
                Err(error) if self.inner.seed_catalog.is_empty() => return Err(error),
                Err(error) => {
                    log::warn!("resource catalog refresh failed; using seed catalog: {error}");
                    None
                }
            }
        } else {
            None
        };
        let fetched_remote = remote.is_some();
        let bytes = if let Some(bytes) = remote {
            bytes
        } else if self.inner.cache_path().is_file() {
            fs::read(self.inner.cache_path()).map_err(operation)?
        } else if !self.inner.seed_catalog.is_empty() {
            self.inner.seed_catalog.as_bytes().to_vec()
        } else {
            return Err(DownloadableResourceError::Unavailable(
                "resource catalog is unavailable; connect to the internet and retry".to_owned(),
            ));
        };
        let document: CatalogDocument = serde_json::from_slice(&bytes).map_err(operation)?;
        let catalog = DownloadableResourceCatalog::from(document);
        validate_catalog(&catalog)?;
        if fetched_remote {
            write_catalog_cache(&self.inner.cache_path(), &bytes)?;
        }
        *self.inner.lock_catalog()? = Some(catalog.clone());
        Ok(catalog)
    }

    async fn fetch_catalog(&self) -> DownloadableResourceResult<Vec<u8>> {
        if !self.inner.catalog_url.starts_with("https://") {
            return Err(DownloadableResourceError::InvalidCatalog(
                "catalog URL must use HTTPS".to_owned(),
            ));
        }
        let response = self
            .inner
            .client
            .get(&self.inner.catalog_url)
            .send()
            .await
            .map_err(operation)?
            .error_for_status()
            .map_err(operation)?;
        Ok(response.bytes().await.map_err(operation)?.to_vec())
    }

    async fn install_descriptor(
        &self,
        descriptor: &DownloadableResourceDescriptor,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> DownloadableResourceResult<DownloadableResourceStatus> {
        if self.inner.ready(descriptor) {
            return Ok(self.inner.status(descriptor));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut cancellations = self.inner.cancellations.lock().map_err(poisoned)?;
            if cancellations.contains_key(&descriptor.id) {
                return Err(DownloadableResourceError::Operation(format!(
                    "resource installation is already active: {}",
                    descriptor.id
                )));
            }
            cancellations.insert(descriptor.id.clone(), cancelled.clone());
        }
        self.inner.activities.lock().map_err(poisoned)?.insert(
            descriptor.id.clone(),
            InstallActivity {
                state: DownloadableResourceState::Downloading,
                downloaded_bytes: 0,
            },
        );
        self.inner
            .failures
            .lock()
            .map_err(poisoned)?
            .remove(&descriptor.id);
        let completion_flag = Arc::new(AtomicBool::new(false));
        let registration = InstallRegistration {
            resource_id: descriptor.id.clone(),
            manager: Arc::downgrade(&self.inner),
            cancelled: cancelled.clone(),
            completed: completion_flag.clone(),
        };
        let staging = self.inner.staging_root(descriptor);
        fs::create_dir_all(&staging).map_err(operation)?;
        let mut completed = 0_u64;
        for file in &descriptor.files {
            let base = completed;
            download_file(
                &self.inner.client,
                file,
                &staging.join(&file.path),
                &cancelled,
                |current| {
                    self.inner.update_activity(
                        &descriptor.id,
                        DownloadableResourceState::Downloading,
                        base.saturating_add(current),
                    );
                    if let Some(sink) = progress {
                        sink.report(ResourceInstallProgress {
                            resource_id: descriptor.id.clone(),
                            downloaded_bytes: base.saturating_add(current),
                            total_bytes: descriptor.size_bytes(),
                        });
                    }
                },
            )
            .await?;
            completed = completed.saturating_add(file.size_bytes);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(DownloadableResourceError::Cancelled);
        }
        self.inner.update_activity(
            &descriptor.id,
            DownloadableResourceState::Verifying,
            descriptor.size_bytes(),
        );
        for file in &descriptor.files {
            verify(&staging.join(&file.path), file)?;
        }
        write_json_atomic(
            &staging.join("resource.json"),
            &InstalledManifest::from(descriptor),
        )?;
        self.activate_descriptor(descriptor, staging)?;
        completion_flag.store(true, Ordering::Release);
        drop(registration);
        Ok(self.inner.status(descriptor))
    }

    fn activate_descriptor(
        &self,
        descriptor: &DownloadableResourceDescriptor,
        staging: PathBuf,
    ) -> DownloadableResourceResult<()> {
        let destination = self.inner.version_root(descriptor);
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(operation)?;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(operation)?;
        }
        fs::rename(staging, &destination).map_err(operation)?;
        let previous = {
            let mut state = self.inner.lock_state()?;
            let mut updated = state.clone();
            let previous = updated
                .active
                .insert(descriptor.id.clone(), descriptor.version.clone());
            if let Err(error) = updated.write(&self.inner.state_path()) {
                let _ = fs::remove_dir_all(&destination);
                return Err(error);
            }
            *state = updated;
            previous
        };
        if let Some(previous) = previous.filter(|version| version != &descriptor.version) {
            self.inner.retire_version(&descriptor.id, &previous)?;
        }
        Ok(())
    }

    async fn install_with_dependencies(
        &self,
        id: &str,
        catalog: &DownloadableResourceCatalog,
        progress: Option<&dyn ResourceInstallProgressSink>,
        installed: &mut HashSet<String>,
    ) -> DownloadableResourceResult<()> {
        let mut pending = vec![id.to_owned()];
        while let Some(current) = pending.pop() {
            if installed.contains(&current) {
                continue;
            }
            let descriptor = find_resource(catalog, &current)?;
            let missing = descriptor
                .dependencies
                .iter()
                .filter(|dependency| !installed.contains(*dependency))
                .cloned()
                .collect::<Vec<_>>();
            if missing.is_empty() {
                self.install_descriptor(descriptor, progress).await?;
                installed.insert(current);
            } else {
                pending.push(current);
                pending.extend(missing.into_iter().rev());
            }
        }
        Ok(())
    }
}

#[async_trait]
impl DownloadableResourceManager for FileSystemDownloadableResourceManager {
    fn onboarding_complete(&self) -> DownloadableResourceResult<bool> {
        Ok(self.inner.lock_state()?.onboarding_complete)
    }

    fn complete_onboarding(&self) -> DownloadableResourceResult<()> {
        let mut state = self.inner.lock_state()?;
        let mut updated = state.clone();
        updated.onboarding_complete = true;
        updated.write(&self.inner.state_path())?;
        *state = updated;
        drop(state);
        Ok(())
    }

    async fn catalog(
        &self,
        refresh: bool,
    ) -> DownloadableResourceResult<DownloadableResourceCatalog> {
        self.load_catalog(refresh).await
    }

    async fn statuses(&self) -> DownloadableResourceResult<Vec<DownloadableResourceStatus>> {
        let catalog = self.load_catalog(false).await?;
        Ok(catalog
            .resources
            .iter()
            .map(|value| self.inner.status(value))
            .collect())
    }

    async fn install(
        &self,
        resource_id: &str,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> DownloadableResourceResult<DownloadableResourceStatus> {
        let catalog = self.load_catalog(false).await?;
        self.install_with_dependencies(resource_id, &catalog, progress, &mut HashSet::new())
            .await?;
        Ok(self.inner.status(find_resource(&catalog, resource_id)?))
    }

    async fn install_group(
        &self,
        group_id: &str,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> DownloadableResourceResult<Vec<DownloadableResourceStatus>> {
        let catalog = self.load_catalog(false).await?;
        let group = catalog
            .groups
            .iter()
            .find(|value| value.id == group_id)
            .ok_or_else(|| {
                DownloadableResourceError::Unavailable(format!(
                    "unknown resource group: {group_id}"
                ))
            })?;
        let mut installed = HashSet::new();
        for resource in &group.resources {
            self.install_with_dependencies(resource, &catalog, progress, &mut installed)
                .await?;
        }
        Ok(group
            .resources
            .iter()
            .map(|id| {
                self.inner
                    .status(find_resource(&catalog, id).expect("validated catalog"))
            })
            .collect())
    }

    fn cancel_install(&self, resource_id: &str) -> DownloadableResourceResult<()> {
        if let Some(cancel) = self
            .inner
            .cancellations
            .lock()
            .map_err(poisoned)?
            .get(resource_id)
        {
            cancel.store(true, Ordering::Release);
        }
        Ok(())
    }

    async fn delete(&self, resource_id: &str) -> DownloadableResourceResult<()> {
        let version = self
            .inner
            .lock_state()?
            .active
            .get(resource_id)
            .cloned()
            .ok_or_else(|| DownloadableResourceError::Unavailable(resource_id.to_owned()))?;
        let key = format!("{resource_id}@{version}");
        let in_use = self
            .inner
            .leases
            .lock()
            .map_err(poisoned)?
            .get(&key)
            .and_then(Weak::upgrade)
            .is_some();
        let mut state = self.inner.lock_state()?;
        let mut updated = state.clone();
        updated.active.remove(resource_id);
        if in_use {
            updated.pending_delete.insert(key);
        }
        updated.write(&self.inner.state_path())?;
        *state = updated;
        drop(state);
        if !in_use {
            remove_version(&self.inner.root, resource_id, &version)?;
        }
        Ok(())
    }

    fn resolve(&self, resource_id: &str) -> DownloadableResourceResult<InstalledResource> {
        let version = self
            .inner
            .lock_state()?
            .active
            .get(resource_id)
            .cloned()
            .ok_or_else(|| DownloadableResourceError::Unavailable(resource_id.to_owned()))?;
        let root = self.inner.root.join(resource_id).join(&version);
        if !root.join("resource.json").is_file() {
            return Err(DownloadableResourceError::Unavailable(
                resource_id.to_owned(),
            ));
        }
        let key = format!("{resource_id}@{version}");
        let lease = {
            let mut leases = self.inner.leases.lock().map_err(poisoned)?;
            let existing = leases.get(&key).and_then(Weak::upgrade);
            let lease = existing.unwrap_or_else(|| {
                let lease = Arc::new(ResourceLease {
                    key: key.clone(),
                    manager: Arc::downgrade(&self.inner),
                });
                leases.insert(key, Arc::downgrade(&lease));
                lease
            });
            drop(leases);
            lease
        };
        Ok(InstalledResource {
            id: resource_id.to_owned(),
            version,
            root,
            lease,
        })
    }
}

impl ManagerInner {
    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }
    fn cache_path(&self) -> PathBuf {
        self.root.join("catalog-v1.json")
    }
    fn staging_root(&self, value: &DownloadableResourceDescriptor) -> PathBuf {
        self.root
            .join(".staging")
            .join(&value.id)
            .join(&value.version)
    }
    fn version_root(&self, value: &DownloadableResourceDescriptor) -> PathBuf {
        self.root.join(&value.id).join(&value.version)
    }
    fn lock_state(&self) -> DownloadableResourceResult<std::sync::MutexGuard<'_, InstalledState>> {
        self.state.lock().map_err(poisoned)
    }
    fn lock_catalog(
        &self,
    ) -> DownloadableResourceResult<std::sync::MutexGuard<'_, Option<DownloadableResourceCatalog>>>
    {
        self.catalog.lock().map_err(poisoned)
    }
    fn ready(&self, value: &DownloadableResourceDescriptor) -> bool {
        self.lock_state()
            .ok()
            .and_then(|state| state.active.get(&value.id).cloned())
            .is_some_and(|version| {
                version == value.version && self.version_root(value).join("resource.json").is_file()
            })
    }
    fn status(&self, value: &DownloadableResourceDescriptor) -> DownloadableResourceStatus {
        let installed = self
            .lock_state()
            .ok()
            .and_then(|state| state.active.get(&value.id).cloned());
        let activity = self
            .activities
            .lock()
            .ok()
            .and_then(|activities| activities.get(&value.id).cloned());
        let failure = self
            .failures
            .lock()
            .ok()
            .and_then(|failures| failures.get(&value.id).cloned());
        let state = activity.as_ref().map_or_else(
            || {
                if installed.as_deref() == Some(value.version.as_str()) {
                    DownloadableResourceState::Ready
                } else if installed.is_some() {
                    DownloadableResourceState::UpdateAvailable
                } else if failure.is_some() {
                    DownloadableResourceState::Failed
                } else {
                    DownloadableResourceState::Missing
                }
            },
            |activity| activity.state,
        );
        DownloadableResourceStatus {
            id: value.id.clone(),
            available_version: value.version.clone(),
            installed_version: installed,
            state,
            size_bytes: value.size_bytes(),
            downloaded_bytes: activity.map_or(0, |value| value.downloaded_bytes),
            message: failure,
        }
    }
    fn finish_pending_delete(&self, key: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !state.pending_delete.contains(key) {
            return;
        }
        if let Some((id, version)) = key.rsplit_once('@') {
            if remove_version(&self.root, id, version).is_err() {
                return;
            }
            let mut updated = state.clone();
            updated.pending_delete.remove(key);
            if updated.write(&self.state_path()).is_ok() {
                *state = updated;
            }
        }
    }

    fn retire_version(&self, id: &str, version: &str) -> DownloadableResourceResult<()> {
        let key = format!("{id}@{version}");
        let in_use = self
            .leases
            .lock()
            .map_err(poisoned)?
            .get(&key)
            .and_then(Weak::upgrade)
            .is_some();
        if in_use {
            let mut state = self.lock_state()?;
            let mut updated = state.clone();
            updated.pending_delete.insert(key);
            updated.write(&self.state_path())?;
            *state = updated;
            drop(state);
            Ok(())
        } else {
            remove_version(&self.root, id, version)
        }
    }

    fn update_activity(&self, id: &str, state: DownloadableResourceState, downloaded_bytes: u64) {
        if let Ok(mut activities) = self.activities.lock()
            && let Some(activity) = activities.get_mut(id)
        {
            activity.state = state;
            activity.downloaded_bytes = downloaded_bytes;
        }
    }
}

struct InstallRegistration {
    resource_id: String,
    manager: Weak<ManagerInner>,
    cancelled: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
}

impl Drop for InstallRegistration {
    fn drop(&mut self) {
        if let Some(manager) = self.manager.upgrade()
            && let Ok(mut cancellations) = manager.cancellations.lock()
        {
            cancellations.remove(&self.resource_id);
            drop(cancellations);
            if let Ok(mut activities) = manager.activities.lock() {
                activities.remove(&self.resource_id);
            }
            if !self.completed.load(Ordering::Acquire)
                && !self.cancelled.load(Ordering::Acquire)
                && let Ok(mut failures) = manager.failures.lock()
            {
                failures.insert(
                    self.resource_id.clone(),
                    "the previous installation did not complete".to_owned(),
                );
            }
        }
    }
}

#[derive(serde::Serialize)]
struct InstalledManifest<'a> {
    format: &'static str,
    schema_version: u32,
    id: &'a str,
    version: &'a str,
    contract_version: u32,
    files: Vec<InstalledFile<'a>>,
}

#[derive(serde::Serialize)]
struct InstalledFile<'a> {
    path: &'a str,
    size_bytes: u64,
    sha256: &'a str,
}

impl<'a> From<&'a DownloadableResourceDescriptor> for InstalledManifest<'a> {
    fn from(value: &'a DownloadableResourceDescriptor) -> Self {
        Self {
            format: "atelier.installed-downloadable-resource",
            schema_version: 1,
            id: &value.id,
            version: &value.version,
            contract_version: value.contract_version,
            files: value
                .files
                .iter()
                .map(|file| InstalledFile {
                    path: &file.path,
                    size_bytes: file.size_bytes,
                    sha256: &file.sha256,
                })
                .collect(),
        }
    }
}

fn find_resource<'a>(
    catalog: &'a DownloadableResourceCatalog,
    id: &str,
) -> DownloadableResourceResult<&'a DownloadableResourceDescriptor> {
    catalog
        .resources
        .iter()
        .find(|value| value.id == id)
        .ok_or_else(|| {
            DownloadableResourceError::Unavailable(format!("unknown downloadable resource: {id}"))
        })
}

fn remove_version(root: &Path, id: &str, version: &str) -> DownloadableResourceResult<()> {
    let target = root.join(id).join(version);
    if target.exists() {
        fs::remove_dir_all(target).map_err(operation)?;
    }
    Ok(())
}

fn write_catalog_cache(path: &Path, bytes: &[u8]) -> DownloadableResourceResult<()> {
    let temporary = path.with_extension("json.part");
    fs::write(&temporary, bytes).map_err(operation)?;
    if path.exists() {
        fs::remove_file(path).map_err(operation)?;
    }
    fs::rename(temporary, path).map_err(operation)
}

fn poisoned<T>(_: std::sync::PoisonError<T>) -> DownloadableResourceError {
    DownloadableResourceError::Operation("downloadable resource state is unavailable".to_owned())
}
