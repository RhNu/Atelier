use async_trait::async_trait;
use nai_atelier_resource_catalog::ResourceRef;

use crate::{SafetyAssessment, SafetyResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyScanInput {
    pub resource: ResourceRef,
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
}

#[async_trait]
pub trait SafetyScanner: Send + Sync {
    async fn scan_image(&self, input: SafetyScanInput) -> SafetyResult<SafetyAssessment>;
}
