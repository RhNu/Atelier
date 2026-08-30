use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_downloadable_resources::{DownloadableResourceManager, InstalledResource};
use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisError, ImageAnalysisInput,
    ImageAnalysisModelId, ImageAnalysisResult, ImageAnalysisSessionControl, ImageAnalyzer,
};

use crate::OrtRuntime;
use crate::analyzer::OnnxImageAnalyzer;
use crate::spec::{ANIME_DBRATING_RESOURCE_ID, WD_TAGGER_RESOURCE_ID};

pub struct OnnxImageAnalysisRuntime {
    runtime: &'static OrtRuntime,
    resources: Arc<dyn DownloadableResourceManager>,
    sessions: Mutex<HashMap<ImageAnalysisModelId, Arc<LoadedAnalyzer>>>,
}

struct LoadedAnalyzer {
    analyzer: OnnxImageAnalyzer,
    _resource: InstalledResource,
}

impl OnnxImageAnalysisRuntime {
    /// Creates an analyzer backed by the shared downloadable-resource manager.
    ///
    /// # Errors
    /// Returns an error when the selected ONNX Runtime does not match the initialized runtime.
    pub fn new(
        runtime: &'static OrtRuntime,
        runtime_library_path: &Path,
        resources: Arc<dyn DownloadableResourceManager>,
    ) -> ImageAnalysisResult<Arc<Self>> {
        runtime
            .for_path(runtime_library_path)
            .map_err(|error| ImageAnalysisError::inference(error.to_string()))?;
        Ok(Arc::new(Self {
            runtime,
            resources,
            sessions: Mutex::new(HashMap::new()),
        }))
    }

    fn analyzer(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<Arc<LoadedAnalyzer>> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ImageAnalysisError::inference("model session state is unavailable"))?;
        if let Some(analyzer) = sessions.get(&model) {
            return Ok(analyzer.clone());
        }
        self.runtime
            .for_path(self.runtime.library_path())
            .map_err(|error| ImageAnalysisError::inference(error.to_string()))?;
        let resource = self
            .resources
            .resolve(resource_id(model))
            .map_err(|error| ImageAnalysisError::model_unavailable(error.to_string()))?;
        let analyzer = match model {
            ImageAnalysisModelId::AnimeDbRating => {
                OnnxImageAnalyzer::load_dbrating(&resource.root.join("model.onnx"))?
            }
            ImageAnalysisModelId::WdSwinv2TaggerV3 => OnnxImageAnalyzer::load_wd(
                &resource.root.join("model.onnx"),
                &resource.root.join("selected_tags.csv"),
            )?,
        };
        let loaded = Arc::new(LoadedAnalyzer {
            analyzer,
            _resource: resource,
        });
        sessions.insert(model, loaded.clone());
        drop(sessions);
        Ok(loaded)
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
        let analyzer = self.analyzer(model)?;
        tokio::task::spawn_blocking(move || analyzer.analyzer.analyze(input, outputs))
            .await
            .map_err(|error| ImageAnalysisError::inference(error.to_string()))?
    }
}

impl ImageAnalysisSessionControl for OnnxImageAnalysisRuntime {
    fn unload(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        self.sessions
            .lock()
            .map_err(|_| ImageAnalysisError::inference("model session state is unavailable"))?
            .remove(&model);
        Ok(())
    }
}

const fn resource_id(model: ImageAnalysisModelId) -> &'static str {
    match model {
        ImageAnalysisModelId::AnimeDbRating => ANIME_DBRATING_RESOURCE_ID,
        ImageAnalysisModelId::WdSwinv2TaggerV3 => WD_TAGGER_RESOURCE_ID,
    }
}
