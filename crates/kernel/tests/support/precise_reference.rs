use async_trait::async_trait;
use atelier_kernel::KernelPreciseReferencePorts;
use atelier_precise_reference::{PreciseReferenceImage, PreciseReferenceResult};
use atelier_resource_catalog::ResourceRef;

use super::MemoryKernelPorts;

#[async_trait]
impl KernelPreciseReferencePorts for MemoryKernelPorts {
    async fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage> {
        self.state
            .lock()
            .unwrap()
            .precise_reference_images
            .get(source.id.as_str())
            .cloned()
            .ok_or_else(|| {
                atelier_precise_reference::PreciseReferenceError::not_found(
                    "precise reference image is missing",
                )
            })
    }
}
