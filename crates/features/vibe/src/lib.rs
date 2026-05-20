mod document;
mod error;
mod model;
mod ports;

pub use document::VibeDocumentCodec;
pub use error::{VibeDomainResult, VibeError, VibeErrorKind};
pub use model::{
    EncodeVibeRequest, EncodedVibe, VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary,
    VibeEncodeSettings, VibeEncodingConfig, VibeEncodingRecord, VibeExportDocument,
    VibeExportEntry, VibeExportFormat, VibeId, VibeImportDocument, VibeImportEntry,
    VibeImportedEncoding, VibeModel, VibeResult, VibeSourceIdentity,
};
pub use ports::{EmbeddedVibeDocumentExtractor, NovelAiVibeClient, VibeRepository};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-vibe");
    }

    #[test]
    fn encode_vibe_request_defaults_to_strict_v45_full() {
        let request = EncodeVibeRequest::default();

        assert_eq!(request.model, VibeModel::NaiDiffusion45Full);
        assert!((request.information_extracted - 1.0).abs() < f32::EPSILON);
        assert!(request.strict_mode);
    }
}
