//! Model-neutral image analysis contracts.

mod error;
mod model;
mod ports;

pub use error::{ImageAnalysisError, ImageAnalysisErrorKind, ImageAnalysisResult};
pub use model::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisInput, ImageAnalysisModelId,
    ImageAnalysisModelInfo, ImageConfidence, ImageRatingScores, ImageTagCategory, ImageTagScore,
};
pub use ports::{ImageAnalysisSessionControl, ImageAnalyzer};
