use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisError, ImageAnalysisInput,
    ImageAnalysisModelId, ImageAnalysisModelManager, ImageAnalysisModelState,
    ImageAnalysisModelStatus, ImageAnalysisResult, ImageAnalyzer, ModelInstallProgress,
    ModelInstallProgressSink,
};
use tokio::sync::Mutex as AsyncMutex;

use crate::OrtRuntime;
use crate::analyzer::OnnxImageAnalyzer;
use crate::manifest::{read_and_validate_manifest, write_manifest};
use crate::spec::{ModelSpec, model_spec};

mod download;

#[derive(Clone)]
pub struct OnnxImageAnalysisRuntime {
    root: PathBuf,
    runtime: &'static OrtRuntime,
    client: reqwest::Client,
    slots: HashMap<ImageAnalysisModelId, Arc<ModelSlot>>,
}

struct ModelSlot {
    install_lock: AsyncMutex<()>,
    installing: AtomicBool,
    verified: AtomicBool,
    cancel: AtomicBool,
    session: Mutex<Option<Arc<OnnxImageAnalyzer>>>,
    last_error: Mutex<Option<String>>,
}

impl Default for ModelSlot {
    fn default() -> Self {
        Self {
            install_lock: AsyncMutex::new(()),
            installing: AtomicBool::new(false),
            verified: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            session: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }
}

impl OnnxImageAnalysisRuntime {
    /// Creates a model runtime rooted in the desktop application's data directory.
    ///
    /// # Errors
    /// Returns an error when the selected ONNX Runtime does not match the initialized runtime.
    pub fn new(
        root: impl Into<PathBuf>,
        runtime: &'static OrtRuntime,
        runtime_library_path: &Path,
    ) -> ImageAnalysisResult<Arc<Self>> {
        runtime
            .for_path(runtime_library_path)
            .map_err(|error| ImageAnalysisError::inference(error.to_string()))?;
        let slots = [
            ImageAnalysisModelId::AnimeDbRating,
            ImageAnalysisModelId::WdSwinv2TaggerV3,
        ]
        .into_iter()
        .map(|id| (id, Arc::new(ModelSlot::default())))
        .collect();
        let client = reqwest::Client::builder()
            .user_agent("Atelier/0.3 image-analysis-model-manager")
            .build()
            .map_err(install_error)?;
        Ok(Arc::new(Self {
            root: root.into(),
            runtime,
            client,
            slots,
        }))
    }

    fn slot(&self, id: ImageAnalysisModelId) -> &Arc<ModelSlot> {
        self.slots
            .get(&id)
            .expect("all supported image analysis model slots must exist")
    }

    fn revision_root(&self, spec: &ModelSpec) -> PathBuf {
        self.root.join(spec.id.as_str()).join(spec.revision)
    }

    fn manifest_path(&self, spec: &ModelSpec) -> PathBuf {
        self.revision_root(spec).join("manifest.json")
    }

    fn inspect(&self, spec: &ModelSpec) -> ImageAnalysisModelStatus {
        let total = total_bytes(spec);
        if self.slot(spec.id).installing.load(Ordering::Acquire) {
            return status(
                spec,
                ImageAnalysisModelState::Installing,
                partial_bytes(&self.revision_root(spec), spec),
                None,
            );
        }
        let manifest_path = self.manifest_path(spec);
        if !manifest_path.is_file() {
            self.slot(spec.id).verified.store(false, Ordering::Release);
            let message = self
                .slot(spec.id)
                .last_error
                .lock()
                .ok()
                .and_then(|value| value.clone());
            return status(
                spec,
                if message.is_some() {
                    ImageAnalysisModelState::Failed
                } else {
                    ImageAnalysisModelState::Missing
                },
                partial_bytes(&self.revision_root(spec), spec),
                message,
            );
        }
        if self.slot(spec.id).verified.load(Ordering::Acquire) {
            return status(spec, ImageAnalysisModelState::Ready, total, None);
        }
        match read_and_validate_manifest(&manifest_path, spec) {
            Ok(()) => {
                self.slot(spec.id).verified.store(true, Ordering::Release);
                status(spec, ImageAnalysisModelState::Ready, total, None)
            }
            Err(error) => status(
                spec,
                ImageAnalysisModelState::Corrupt,
                0,
                Some(error.to_string()),
            ),
        }
    }

    async fn inspect_async(
        &self,
        id: ImageAnalysisModelId,
    ) -> ImageAnalysisResult<ImageAnalysisModelStatus> {
        let runtime = self.clone();
        tokio::task::spawn_blocking(move || runtime.inspect(model_spec(id)))
            .await
            .map_err(blocking_task_error)
    }

    async fn ensure_primary_ready(&self) -> ImageAnalysisResult<()> {
        if !self.is_ready(ImageAnalysisModelId::AnimeDbRating).await {
            self.install(ImageAnalysisModelId::AnimeDbRating, None)
                .await?;
        }
        Ok(())
    }

    fn analyzer(&self, id: ImageAnalysisModelId) -> ImageAnalysisResult<Arc<OnnxImageAnalyzer>> {
        let slot = self.slot(id);
        let mut session = slot
            .session
            .lock()
            .map_err(|_| ImageAnalysisError::inference("model session lock is unavailable"))?;
        if let Some(analyzer) = session.as_ref() {
            return Ok(Arc::clone(analyzer));
        }
        let spec = model_spec(id);
        if self.inspect(spec).state != ImageAnalysisModelState::Ready {
            return Err(ImageAnalysisError::model_unavailable(
                "image analysis model package is not ready",
            ));
        }
        self.runtime
            .for_path(self.runtime.library_path())
            .map_err(|error| ImageAnalysisError::inference(error.to_string()))?;
        let root = self.revision_root(spec);
        let analyzer = match id {
            ImageAnalysisModelId::AnimeDbRating => {
                OnnxImageAnalyzer::load_dbrating(&root.join("model.onnx"))?
            }
            ImageAnalysisModelId::WdSwinv2TaggerV3 => OnnxImageAnalyzer::load_wd(
                &root.join("model.onnx"),
                &root.join("selected_tags.csv"),
            )?,
        };
        let analyzer = Arc::new(analyzer);
        *session = Some(Arc::clone(&analyzer));
        drop(session);
        Ok(analyzer)
    }

    fn delete_model_files(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        let spec = model_spec(model);
        self.unload(model)?;
        self.slot(model).verified.store(false, Ordering::Release);
        let root = self.revision_root(spec);
        if !root.exists() {
            return Ok(());
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let deleting = root.with_file_name(format!("{}.deleting-{timestamp}", spec.revision));
        fs::rename(&root, &deleting).map_err(install_error)?;
        fs::remove_dir_all(deleting).map_err(install_error)
    }

    async fn install_files(
        &self,
        spec: &ModelSpec,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> ImageAnalysisResult<()> {
        let mut completed = 0;
        for file in spec.files {
            self.download_file(spec, file, completed, progress).await?;
            completed += file.size_bytes;
        }
        let runtime = self.clone();
        let spec = *spec;
        tokio::task::spawn_blocking(move || {
            let manifest_path = runtime.manifest_path(&spec);
            write_manifest(&manifest_path, &spec)?;
            read_and_validate_manifest(&manifest_path, &spec)
        })
        .await
        .map_err(blocking_task_error)?
    }
}

#[async_trait]
impl ImageAnalyzer for OnnxImageAnalysisRuntime {
    async fn analyze(
        &self,
        model: ImageAnalysisModelId,
        input: ImageAnalysisInput,
        outputs: AnalysisOutputSelection,
    ) -> ImageAnalysisResult<ImageAnalysis> {
        if model == ImageAnalysisModelId::AnimeDbRating {
            self.ensure_primary_ready().await?;
        } else if !self.is_ready(model).await {
            return Err(ImageAnalysisError::model_unavailable(
                "optional WD Tagger model is not installed",
            ));
        }
        let runtime = self.clone();
        tokio::task::spawn_blocking(move || runtime.analyzer(model)?.analyze(input, outputs))
            .await
            .map_err(blocking_task_error)?
    }
}

#[async_trait]
impl ImageAnalysisModelManager for OnnxImageAnalysisRuntime {
    async fn statuses(&self) -> ImageAnalysisResult<Vec<ImageAnalysisModelStatus>> {
        let runtime = self.clone();
        tokio::task::spawn_blocking(move || {
            [
                ImageAnalysisModelId::AnimeDbRating,
                ImageAnalysisModelId::WdSwinv2TaggerV3,
            ]
            .into_iter()
            .map(|id| runtime.inspect(model_spec(id)))
            .collect()
        })
        .await
        .map_err(blocking_task_error)
    }

    async fn install(
        &self,
        model: ImageAnalysisModelId,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> ImageAnalysisResult<ImageAnalysisModelStatus> {
        let spec = model_spec(model);
        let slot = self.slot(model);
        let _guard = slot.install_lock.lock().await;
        let status = self.inspect_async(model).await?;
        if status.state == ImageAnalysisModelState::Ready {
            return Ok(status);
        }
        slot.installing.store(true, Ordering::Release);
        let _installing = InstallingGuard(&slot.installing);
        slot.cancel.store(false, Ordering::Release);
        set_last_error(slot, String::new());
        if let Err(error) = self.install_files(spec, progress).await {
            set_last_error(slot, error.to_string());
            return Err(error);
        }
        set_last_error(slot, String::new());
        slot.verified.store(true, Ordering::Release);
        drop(_installing);
        self.inspect_async(model).await
    }

    fn cancel_install(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        self.slot(model).cancel.store(true, Ordering::Release);
        Ok(())
    }

    async fn delete(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        let spec = model_spec(model);
        if spec.required {
            return Err(ImageAnalysisError::invalid_request(
                "the required dbrating model cannot be deleted",
            ));
        }
        let slot = self.slot(model);
        let _guard = slot.install_lock.lock().await;
        let runtime = self.clone();
        tokio::task::spawn_blocking(move || runtime.delete_model_files(model))
            .await
            .map_err(blocking_task_error)?
    }

    fn unload(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        let analyzer = self
            .slot(model)
            .session
            .lock()
            .map_err(|_| ImageAnalysisError::inference("model session lock is unavailable"))?
            .take();
        analyzer.map_or(Ok(()), |analyzer| analyzer.wait_until_idle())
    }

    async fn is_ready(&self, model: ImageAnalysisModelId) -> bool {
        self.inspect_async(model)
            .await
            .is_ok_and(|status| status.state == ImageAnalysisModelState::Ready)
    }
}

struct InstallingGuard<'a>(&'a AtomicBool);

impl Drop for InstallingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn status(
    spec: &ModelSpec,
    state: ImageAnalysisModelState,
    downloaded_bytes: u64,
    message: Option<String>,
) -> ImageAnalysisModelStatus {
    ImageAnalysisModelStatus {
        id: spec.id,
        required: spec.required,
        state,
        revision: spec.revision.to_owned(),
        size_bytes: total_bytes(spec),
        downloaded_bytes,
        message: message.filter(|message| !message.is_empty()),
    }
}

fn report_progress(
    sink: Option<&dyn ModelInstallProgressSink>,
    id: ImageAnalysisModelId,
    downloaded_bytes: u64,
    total_bytes: u64,
) {
    if let Some(sink) = sink {
        sink.report(ModelInstallProgress {
            id,
            downloaded_bytes: downloaded_bytes.min(total_bytes),
            total_bytes,
        });
    }
}

fn total_bytes(spec: &ModelSpec) -> u64 {
    spec.files.iter().map(|file| file.size_bytes).sum()
}

fn partial_bytes(root: &Path, spec: &ModelSpec) -> u64 {
    spec.files
        .iter()
        .map(|file| {
            let final_path = root.join(file.relative_path);
            let partial_path = root.join(format!("{}.part", file.relative_path));
            fs::metadata(final_path)
                .or_else(|_| fs::metadata(partial_path))
                .map_or(0, |metadata| metadata.len().min(file.size_bytes))
        })
        .sum()
}

fn set_last_error(slot: &ModelSlot, message: String) {
    if let Ok(mut last_error) = slot.last_error.lock() {
        *last_error = (!message.is_empty()).then_some(message);
    }
}

fn install_error(error: impl std::fmt::Display) -> ImageAnalysisError {
    ImageAnalysisError::model_install(error.to_string())
}

fn blocking_task_error(error: impl std::fmt::Display) -> ImageAnalysisError {
    ImageAnalysisError::inference(format!("blocking image-analysis task failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_byte_count_is_bounded_by_pinned_file_size() {
        let temp = tempfile::tempdir().unwrap();
        let files = Box::leak(Box::new([crate::spec::ModelFileSpec {
            relative_path: "model.onnx",
            url: "https://example.invalid/model.onnx",
            sha256: "unused",
            size_bytes: 4,
        }]));
        let spec = ModelSpec {
            id: ImageAnalysisModelId::AnimeDbRating,
            revision: "test-revision",
            required: true,
            files,
        };
        fs::write(temp.path().join("model.onnx.part"), b"too many bytes").unwrap();

        assert_eq!(partial_bytes(temp.path(), &spec), 4);
    }
}
