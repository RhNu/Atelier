use super::FileSystemDownloadableResourceManager;
use crate::state::operation;
use atelier_downloadable_resources::{DownloadableResourceError, DownloadableResourceResult};
use std::fs;
use std::path::Path;
impl FileSystemDownloadableResourceManager {
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
}
