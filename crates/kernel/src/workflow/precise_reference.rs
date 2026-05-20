use nai_atelier_generation::CharacterReference;
use nai_atelier_precise_reference::{
    PreciseReferenceImage, PreciseReferenceImageReader, PreciseReferenceInput,
    PreciseReferenceResult, PreciseReferenceService,
};
use nai_atelier_resource_catalog::ResourceRef;

use crate::{KernelPreciseReferencePorts, KernelResult, KernelRuntime};

pub async fn prepare_precise_reference<P>(
    runtime: &KernelRuntime<P>,
    input: PreciseReferenceInput,
) -> KernelResult<CharacterReference>
where
    P: KernelPreciseReferencePorts,
{
    let image = runtime
        .ports()
        .read_precise_reference_image(&input.source)
        .await?;
    let service = PreciseReferenceService::new(ResolvedPreciseReferenceImage { image });
    service.prepare(&input).map_err(Into::into)
}

struct ResolvedPreciseReferenceImage {
    image: PreciseReferenceImage,
}

impl PreciseReferenceImageReader for ResolvedPreciseReferenceImage {
    fn read_precise_reference_image(
        &self,
        _source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage> {
        Ok(self.image.clone())
    }
}
