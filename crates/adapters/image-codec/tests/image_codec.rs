use async_trait::async_trait;
use atelier_adapter_image_codec::{
    ImageCodec, ImageCodecVariantBuilder, ImageExportFormat, ImageMetadataBlobStore,
    ImageSourceReader, StaticImageVariantSettings,
};
use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, ResourceBlobStore, ResourceCatalogErrorKind,
    ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata, ResourceRecord, ResourceRef,
    ResourceResult, ResourceState, ResourceVariantBuilder, ResourceVariantKind, StagedBlob,
    StagedBlobToken, VariantId,
};
use atelier_settings::ImageVariantSettings;
use futures_executor::block_on;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use std::io::Cursor;

#[test]
fn probes_png_jpeg_and_webp_bytes_and_rejects_invalid_input() {
    let png = image_bytes(640, 320, ImageFormat::Png);
    let jpeg = image_bytes(320, 640, ImageFormat::Jpeg);
    let webp = webp_bytes(128, 96);

    assert_eq!(
        ImageCodec::probe(&png).unwrap(),
        image_info("image/png", 640, 320)
    );
    assert_eq!(
        ImageCodec::probe(&jpeg).unwrap(),
        image_info("image/jpeg", 320, 640)
    );
    assert_eq!(
        ImageCodec::probe(&webp).unwrap(),
        image_info("image/webp", 128, 96)
    );
    assert!(ImageCodec::probe(&[1, 2, 3]).is_err());
}

#[test]
fn builds_webp_thumbnail_and_preview_without_upscaling() {
    let source = image_bytes(1200, 600, ImageFormat::Png);
    let settings = ImageVariantSettings {
        thumbnail_long_edge: 320,
        preview_long_edge: 1024,
    };

    let thumbnail =
        ImageCodec::build_variant(&source, ResourceVariantKind::Thumbnail, settings).unwrap();
    assert_eq!(thumbnail.mime_type, "image/webp");
    assert_eq!((thumbnail.width, thumbnail.height), (320, 160));
    assert_eq!(
        ImageCodec::probe(&thumbnail.bytes).unwrap(),
        image_info("image/webp", 320, 160)
    );

    let preview =
        ImageCodec::build_variant(&source, ResourceVariantKind::Preview, settings).unwrap();
    assert_eq!(preview.mime_type, "image/webp");
    assert_eq!((preview.width, preview.height), (1024, 512));

    let small = image_bytes(80, 60, ImageFormat::Png);
    let small_thumbnail =
        ImageCodec::build_variant(&small, ResourceVariantKind::Thumbnail, settings).unwrap();
    assert_eq!((small_thumbnail.width, small_thumbnail.height), (80, 60));
}

#[test]
fn builds_png_sanitized_and_export_at_original_dimensions() {
    let source = image_bytes(640, 360, ImageFormat::Png);
    let settings = ImageVariantSettings::default();

    for kind in [ResourceVariantKind::Sanitized, ResourceVariantKind::Export] {
        let variant = ImageCodec::build_variant(&source, kind, settings).unwrap();
        assert_eq!(variant.mime_type, "image/png");
        assert_eq!((variant.width, variant.height), (640, 360));
        assert_eq!(
            ImageCodec::probe(&variant.bytes).unwrap(),
            image_info("image/png", 640, 360)
        );
    }
}

#[test]
fn exports_original_png_sanitized_png_and_jpeg_at_original_dimensions() {
    let png = image_bytes(48, 32, ImageFormat::Png);
    let original = ImageCodec::encode_export(&png, ImageExportFormat::PngOriginal).unwrap();
    assert_eq!(original.bytes, png);
    assert_eq!(original.mime_type, "image/png");

    let sanitized = ImageCodec::encode_export(&png, ImageExportFormat::PngSanitized).unwrap();
    assert_eq!(
        ImageCodec::probe(&sanitized.bytes).unwrap(),
        image_info("image/png", 48, 32)
    );

    let jpeg = ImageCodec::encode_export(&png, ImageExportFormat::Jpeg).unwrap();
    assert_eq!(
        ImageCodec::probe(&jpeg.bytes).unwrap(),
        image_info("image/jpeg", 48, 32)
    );
    let pixels = ImageCodec::decode_rgba(&jpeg.bytes).unwrap();
    assert_eq!((pixels.width, pixels.height), (48, 32));
    assert_eq!(pixels.bytes.len(), 48 * 32 * 4);
}

#[test]
fn png_original_losslessly_converts_non_png_sources() {
    let webp = webp_bytes(21, 13);
    let exported = ImageCodec::encode_export(&webp, ImageExportFormat::PngOriginal).unwrap();

    assert_eq!(
        ImageCodec::probe(&exported.bytes).unwrap(),
        image_info("image/png", 21, 13)
    );
}

#[test]
fn corrupt_source_maps_to_variant_builder_error() {
    block_on(async {
        let reader = MemoryImageReader {
            bytes: vec![1, 2, 3],
        };
        let builder = ImageCodecVariantBuilder::new(
            reader,
            StaticImageVariantSettings::new(ImageVariantSettings::default()),
        );

        let error = builder
            .build_variant(BuildVariantRequest {
                source: ResourceRef::base(ResourceId::new("source")),
                source_record: source_record(),
                variant_id: VariantId::new("source:thumbnail"),
                kind: ResourceVariantKind::Thumbnail,
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, ResourceCatalogErrorKind::VariantBuilder);
    });
}

#[test]
fn metadata_blob_store_adds_decodable_image_metadata() {
    block_on(async {
        let store = ImageMetadataBlobStore::new(MemoryBlobStore);
        let staged = store
            .stage_blob(BlobWriteIntent::Bytes(image_bytes(
                33,
                17,
                ImageFormat::Png,
            )))
            .await
            .unwrap();

        assert_eq!(staged.metadata.mime_type.as_deref(), Some("image/png"));
        assert_eq!(staged.metadata.width, Some(33));
        assert_eq!(staged.metadata.height, Some(17));
        assert_eq!(staged.metadata.byte_size, Some(1));
    });
}

#[derive(Clone)]
struct MemoryImageReader {
    bytes: Vec<u8>,
}

#[async_trait]
impl ImageSourceReader for MemoryImageReader {
    async fn read_image_source_bytes(&self, _source: &ResourceRef) -> ResourceResult<Vec<u8>> {
        Ok(self.bytes.clone())
    }
}

#[derive(Clone)]
struct MemoryBlobStore;

#[async_trait]
impl ResourceBlobStore for MemoryBlobStore {
    async fn stage_blob(&self, _intent: BlobWriteIntent) -> ResourceResult<StagedBlob> {
        Ok(StagedBlob {
            token: StagedBlobToken::new("staged"),
            blob_id: BlobId::new("sha256:blob"),
            metadata: ResourceMetadata {
                byte_size: Some(1),
                ..ResourceMetadata::default()
            },
        })
    }

    async fn finalize_blob(&self, _staged: &StagedBlobToken) -> ResourceResult<()> {
        Ok(())
    }

    async fn abort_staged_blob(&self, _staged: &StagedBlobToken) -> ResourceResult<()> {
        Ok(())
    }

    async fn delete_blob(&self, _blob_id: &BlobId) -> ResourceResult<()> {
        Ok(())
    }

    async fn blob_exists(&self, _blob_id: &BlobId) -> ResourceResult<bool> {
        Ok(true)
    }
}

fn image_bytes(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = solid_image(width, height);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

fn webp_bytes(width: u32, height: u32) -> Vec<u8> {
    let image = solid_image(width, height).into_rgba8();
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .encode(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    bytes
}

fn solid_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgba8(ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            127,
            255,
        ])
    }))
}

fn image_info(mime_type: &str, width: u32, height: u32) -> atelier_adapter_image_codec::ImageInfo {
    atelier_adapter_image_codec::ImageInfo {
        mime_type: mime_type.to_owned(),
        width,
        height,
    }
}

fn source_record() -> ResourceRecord {
    ResourceRecord {
        id: ResourceId::new("source"),
        kind: ResourceKind::GeneratedImage,
        lifecycle: ResourceLifecycle::JobScoped,
        state: ResourceState::Ready,
        blob_id: atelier_resource_catalog::BlobId::new("sha256:source"),
        metadata: ResourceMetadata::default(),
    }
}
