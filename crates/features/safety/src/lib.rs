//! Simple safety scoring contracts for image resources.

mod error;
mod model;
mod ports;

pub use error::{SafetyError, SafetyErrorKind, SafetyResult};
pub use model::{ImageSafetyScore, SafetyAssessment};
pub use ports::SafetyScanner;
