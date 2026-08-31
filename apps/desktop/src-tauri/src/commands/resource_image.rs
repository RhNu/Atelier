use base64::{Engine as _, engine::general_purpose::STANDARD};

use atelier_adapter_image_codec::{DecodedRgbaImage, ImageCodec, ImageExportFormat};
use atelier_app::CommandResult;
use atelier_app_api::{error::ErrorEnvelopeDto, resource::ImageExportFormatDto};

pub fn decode_resource_image(image_base64: &str) -> CommandResult<Vec<u8>> {
    STANDARD
        .decode(image_base64.trim())
        .map_err(|error| ErrorEnvelopeDto::new("resource_decode_error", error.to_string()))
}

pub fn encode_resource_image(
    source_bytes: &[u8],
    source_mime_type: Option<&str>,
    format: Option<ImageExportFormatDto>,
) -> CommandResult<(Vec<u8>, String)> {
    let Some(format) = format else {
        return Ok((
            source_bytes.to_vec(),
            source_mime_type.unwrap_or("image/png").to_owned(),
        ));
    };
    let format = match format {
        ImageExportFormatDto::PngOriginal => ImageExportFormat::PngOriginal,
        ImageExportFormatDto::PngSanitized => ImageExportFormat::PngSanitized,
        ImageExportFormatDto::Jpeg => ImageExportFormat::Jpeg,
    };
    let encoded = ImageCodec::encode_export(source_bytes, format)
        .map_err(|error| ErrorEnvelopeDto::new("resource_image_encode_error", error.to_string()))?;
    Ok((encoded.bytes, encoded.mime_type))
}

pub fn decode_clipboard_pixels(encoded: &[u8]) -> CommandResult<DecodedRgbaImage> {
    ImageCodec::decode_rgba(encoded)
        .map_err(|error| ErrorEnvelopeDto::new("resource_image_decode_error", error.to_string()))
}
