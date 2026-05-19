use async_trait::async_trait;
use nai_atelier_resource_catalog::ResourceRef;

use crate::{SafetyAssessment, SafetyResult};

#[async_trait]
pub trait SafetyScanner: Send + Sync {
    async fn score_image(&self, resource: ResourceRef) -> SafetyResult<SafetyAssessment>;
}
