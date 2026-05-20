//! Image metadata and variant codec adapter.

use std::io::Cursor;

use async_trait::async_trait;
use image::ImageEncoder;
use nai_atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, BuiltResourceVariant, ResourceBlobStore,
    ResourceCatalogError, ResourceRef, ResourceResult, ResourceVariantBuilder, ResourceVariantKind,
    StagedBlob, StagedBlobToken,
};
use nai_atelier_settings::ImageVariantSettings;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct DecodedImageSource {
    image: image::DynamicImage,
}

impl DecodedImageSource {
    /// Builds a gallery/export variant from an already-decoded source.
    ///
    /// # Errors
    /// Returns an error when target encoding fails.
    pub fn build_variant(
        &self,
        kind: ResourceVariantKind,
        settings: ImageVariantSettings,
    ) -> ImageCodecResult<EncodedImage> {
        match kind {
            ResourceVariantKind::Thumbnail => {
                encode_webp_resized(&self.image, settings.thumbnail_long_edge)
            }
            ResourceVariantKind::Preview => {
                encode_webp_resized(&self.image, settings.preview_long_edge)
            }
            ResourceVariantKind::Sanitized | ResourceVariantKind::Export => encode_png(&self.image),
            ResourceVariantKind::Original => Err(ImageCodecError::new(
                "original is not a derived image variant",
            )),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("image codec: {message}")]
pub struct ImageCodecError {
    message: String,
}

impl ImageCodecError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub type ImageCodecResult<T> = Result<T, ImageCodecError>;

#[derive(Copy, Clone, Debug, Default)]
pub struct ImageCodec;

impl ImageCodec {
    /// Probes supported image bytes.
    ///
    /// # Errors
    /// Returns an error for unsupported formats, corrupt bytes, or decode failures.
    pub fn probe(bytes: &[u8]) -> ImageCodecResult<ImageInfo> {
        let reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|error| {
                ImageCodecError::new(format!("unsupported or corrupt image: {error}"))
            })?;
        let format = reader
            .format()
            .ok_or_else(|| ImageCodecError::new("unsupported image format"))?;
        ensure_supported_format(format)?;
        let (width, height) = reader
            .into_dimensions()
            .map_err(|error| ImageCodecError::new(format!("failed to probe image: {error}")))?;
        Ok(ImageInfo {
            mime_type: mime_type(format).to_owned(),
            width,
            height,
        })
    }

    /// Decodes supported image bytes once for repeated variant builds.
    ///
    /// # Errors
    /// Returns an error for unsupported formats, corrupt bytes, or decode failures.
    pub fn decode_source(bytes: &[u8]) -> ImageCodecResult<DecodedImageSource> {
        let format = supported_format(bytes)?;
        Ok(DecodedImageSource {
            image: decode(bytes, format)?,
        })
    }

    /// Builds a gallery/export variant from supported image bytes.
    ///
    /// # Errors
    /// Returns an error when source decoding or target encoding fails.
    pub fn build_variant(
        bytes: &[u8],
        kind: ResourceVariantKind,
        settings: ImageVariantSettings,
    ) -> ImageCodecResult<EncodedImage> {
        Self::decode_source(bytes)?.build_variant(kind, settings)
    }
}

#[async_trait]
pub trait ImageSourceReader: Send + Sync {
    async fn read_image_source_bytes(&self, source: &ResourceRef) -> ResourceResult<Vec<u8>>;
}

pub trait ImageVariantSettingsProvider: Send + Sync {
    fn image_variant_settings(&self) -> ImageVariantSettings;
}

#[derive(Copy, Clone, Debug)]
pub struct StaticImageVariantSettings {
    settings: ImageVariantSettings,
}

impl StaticImageVariantSettings {
    #[must_use]
    pub const fn new(settings: ImageVariantSettings) -> Self {
        Self { settings }
    }
}

impl ImageVariantSettingsProvider for StaticImageVariantSettings {
    fn image_variant_settings(&self) -> ImageVariantSettings {
        self.settings
    }
}

#[derive(Clone, Debug)]
pub struct ImageCodecVariantBuilder<R, S> {
    reader: R,
    settings: S,
}

impl<R, S> ImageCodecVariantBuilder<R, S> {
    #[must_use]
    pub const fn new(reader: R, settings: S) -> Self {
        Self { reader, settings }
    }
}

#[async_trait]
impl<R, S> ResourceVariantBuilder for ImageCodecVariantBuilder<R, S>
where
    R: ImageSourceReader,
    S: ImageVariantSettingsProvider,
{
    async fn build_variant(
        &self,
        request: BuildVariantRequest,
    ) -> ResourceResult<BuiltResourceVariant> {
        let bytes = self.reader.read_image_source_bytes(&request.source).await?;
        let encoded =
            ImageCodec::build_variant(&bytes, request.kind, self.settings.image_variant_settings())
                .map_err(resource_variant_error)?;
        Ok(BuiltResourceVariant {
            blob: BlobWriteIntent::Bytes(encoded.bytes),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ImageMetadataBlobStore<B> {
    inner: B,
}

impl<B> ImageMetadataBlobStore<B> {
    #[must_use]
    pub const fn new(inner: B) -> Self {
        Self { inner }
    }

    #[must_use]
    pub const fn inner(&self) -> &B {
        &self.inner
    }
}

#[async_trait]
impl<B> ResourceBlobStore for ImageMetadataBlobStore<B>
where
    B: ResourceBlobStore,
{
    async fn stage_blob(&self, intent: BlobWriteIntent) -> ResourceResult<StagedBlob> {
        let BlobWriteIntent::Bytes(bytes) = intent;
        let info = ImageCodec::probe(&bytes).ok();
        let mut staged = self.inner.stage_blob(BlobWriteIntent::Bytes(bytes)).await?;
        if let Some(info) = info {
            staged.metadata.mime_type = Some(info.mime_type);
            staged.metadata.width = Some(info.width);
            staged.metadata.height = Some(info.height);
        }
        Ok(staged)
    }

    async fn finalize_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        self.inner.finalize_blob(staged).await
    }

    async fn abort_staged_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        self.inner.abort_staged_blob(staged).await
    }

    async fn delete_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        self.inner.delete_blob(blob_id).await
    }

    async fn blob_exists(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        self.inner.blob_exists(blob_id).await
    }
}

fn supported_format(bytes: &[u8]) -> ImageCodecResult<image::ImageFormat> {
    let format = image::guess_format(bytes)
        .map_err(|error| ImageCodecError::new(format!("unsupported or corrupt image: {error}")))?;
    ensure_supported_format(format)?;
    Ok(format)
}

fn ensure_supported_format(format: image::ImageFormat) -> ImageCodecResult<()> {
    match format {
        image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP => Ok(()),
        _ => Err(ImageCodecError::new(format!(
            "unsupported image format {format:?}"
        ))),
    }
}

fn decode(bytes: &[u8], format: image::ImageFormat) -> ImageCodecResult<image::DynamicImage> {
    image::load_from_memory_with_format(bytes, format)
        .map_err(|error| ImageCodecError::new(format!("failed to decode image: {error}")))
}

fn encode_webp_resized(
    image: &image::DynamicImage,
    max_long_edge: u32,
) -> ImageCodecResult<EncodedImage> {
    if max_long_edge == 0 {
        return Err(ImageCodecError::new("variant long edge must be non-zero"));
    }
    let (width, height) = scaled_dimensions(image.width(), image.height(), max_long_edge);
    let output = if width == image.width() && height == image.height() {
        image.clone()
    } else {
        image.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    };
    let rgba = output.to_rgba8();
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .encode(
            rgba.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| ImageCodecError::new(format!("failed to encode WebP: {error}")))?;
    Ok(EncodedImage {
        bytes,
        mime_type: "image/webp".to_owned(),
        width,
        height,
    })
}

fn encode_png(image: &image::DynamicImage) -> ImageCodecResult<EncodedImage> {
    let rgba = image.to_rgba8();
    let width = image.width();
    let height = image.height();
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            rgba.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| ImageCodecError::new(format!("failed to encode PNG: {error}")))?;
    Ok(EncodedImage {
        bytes,
        mime_type: "image/png".to_owned(),
        width,
        height,
    })
}

fn scaled_dimensions(width: u32, height: u32, max_long_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_long_edge {
        return (width, height);
    }
    let longest = u64::from(longest);
    let max_long_edge = u64::from(max_long_edge);
    let scaled_width = ((u64::from(width) * max_long_edge) + (longest / 2)) / longest;
    let scaled_height = ((u64::from(height) * max_long_edge) + (longest / 2)) / longest;
    (
        u32::try_from(scaled_width.max(1)).unwrap_or(u32::MAX),
        u32::try_from(scaled_height.max(1)).unwrap_or(u32::MAX),
    )
}

const fn mime_type(format: image::ImageFormat) -> &'static str {
    match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => "application/octet-stream",
    }
}

fn resource_variant_error(error: ImageCodecError) -> ResourceCatalogError {
    let message = error.message;
    ResourceCatalogError::variant_builder(format!("image codec: {message}"))
}
