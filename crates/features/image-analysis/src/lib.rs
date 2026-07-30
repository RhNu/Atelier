//! Model-neutral image analysis contracts.

mod error;
mod model;
mod ports;

pub use error::{ImageAnalysisError, ImageAnalysisErrorKind, ImageAnalysisResult};
pub use model::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisInput, ImageAnalysisModelId,
    ImageAnalysisModelInfo, ImageAnalysisModelState, ImageAnalysisModelStatus, ImageConfidence,
    ImageRatingScores, ImageTagCategory, ImageTagScore, ModelInstallProgress,
};
pub use ports::{ImageAnalysisModelManager, ImageAnalyzer, ModelInstallProgressSink};
