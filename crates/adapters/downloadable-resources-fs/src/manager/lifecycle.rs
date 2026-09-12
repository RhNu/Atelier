use super::{ManagerInner, poisoned, remove_version};
use crate::state::InstalledState;
use atelier_downloadable_resources::{
    DownloadableResourceCatalog, DownloadableResourceDescriptor, DownloadableResourceResult,
    DownloadableResourceState, DownloadableResourceStatus,
};
use std::path::PathBuf;
use std::sync::Weak;
impl ManagerInner {
    pub(super) fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }
    pub(super) fn cache_path(&self) -> PathBuf {
        self.root.join("catalog-v1.json")
    }
    pub(super) fn staging_root(&self, value: &DownloadableResourceDescriptor) -> PathBuf {
        self.root
            .join(".staging")
            .join(&value.id)
            .join(&value.version)
    }
    pub(super) fn version_root(&self, value: &DownloadableResourceDescriptor) -> PathBuf {
        self.root.join(&value.id).join(&value.version)
    }
    pub(super) fn lock_state(
        &self,
    ) -> DownloadableResourceResult<std::sync::MutexGuard<'_, InstalledState>> {
        self.state.lock().map_err(poisoned)
    }
    pub(super) fn lock_catalog(
        &self,
    ) -> DownloadableResourceResult<std::sync::MutexGuard<'_, Option<DownloadableResourceCatalog>>>
    {
        self.catalog.lock().map_err(poisoned)
    }
    pub(super) fn ready(&self, value: &DownloadableResourceDescriptor) -> bool {
        self.lock_state()
            .ok()
            .and_then(|state| state.active.get(&value.id).cloned())
            .is_some_and(|version| {
                version == value.version && self.version_root(value).join("resource.json").is_file()
            })
    }
    pub(super) fn status(
        &self,
        value: &DownloadableResourceDescriptor,
    ) -> DownloadableResourceStatus {
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
    pub(super) fn finish_pending_delete(&self, key: &str) {
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

    pub(super) fn retire_version(&self, id: &str, version: &str) -> DownloadableResourceResult<()> {
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

    pub(super) fn update_activity(
        &self,
        id: &str,
        state: DownloadableResourceState,
        downloaded_bytes: u64,
    ) {
        if let Ok(mut activities) = self.activities.lock()
            && let Some(activity) = activities.get_mut(id)
        {
            activity.state = state;
            activity.downloaded_bytes = downloaded_bytes;
        }
    }
}
