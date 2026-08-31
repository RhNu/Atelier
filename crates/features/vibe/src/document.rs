use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

use crate::{
    VibeDocumentSummary, VibeDomainResult, VibeEncodeSettings, VibeEncodingConfig, VibeError,
    VibeExportDocument, VibeExportEntry, VibeExportFormat, VibeId, VibeImportDocument,
    VibeImportEntry, VibeImportedEncoding, VibeModel,
};

const OFFICIAL_VIBE_IDENTIFIER: &str = "novelai-vibe-transfer";
const OFFICIAL_VIBE_BUNDLE_IDENTIFIER: &str = "novelai-vibe-transfer-bundle";

// Independent codec for the official Vibe Transfer JSON fields used by NAI
// Atelier; reference-project parser implementations are not copied here.
pub struct VibeDocumentCodec;

impl VibeDocumentCodec {
    /// Imports official `.naiv4vibe` or `.naiv4vibebundle` JSON text.
    ///
    /// # Errors
    /// Returns an error when the text is not valid official Vibe Transfer JSON.
    pub fn import_text(file_name: &str, text: &str) -> VibeDomainResult<VibeImportDocument> {
        let parsed: Value = serde_json::from_str(text.trim())
            .map_err(|error| VibeError::invalid_document(error.to_string()))?;
        let root = parsed
            .as_object()
            .ok_or_else(|| VibeError::invalid_document("vibe file content must be an object"))?;
        let entries = match root.get("identifier").and_then(Value::as_str) {
            Some(OFFICIAL_VIBE_IDENTIFIER) => {
                vec![validate_official_vibe_object(&parsed, file_name)?]
            }
            Some(OFFICIAL_VIBE_BUNDLE_IDENTIFIER) => import_bundle(root, file_name)?,
            _ => {
                return Err(VibeError::invalid_document(
                    "only official .naiv4vibe/.naiv4vibebundle formats are supported",
                ));
            }
        };
        Ok(VibeImportDocument { entries })
    }

    /// Exports managed Vibe documents as official JSON text.
    ///
    /// # Errors
    /// Returns an error when the format cannot represent the supplied entries.
    pub fn export_text(
        entries: &[VibeExportEntry],
        format: VibeExportFormat,
    ) -> VibeDomainResult<VibeExportDocument> {
        if entries.is_empty() {
            return Err(VibeError::invalid_document(
                "vibe export requires at least one entry",
            ));
        }
        let (file_extension, value) = match format {
            VibeExportFormat::Naiv4vibe => {
                if entries.len() != 1 {
                    return Err(VibeError::invalid_document(
                        "naiv4vibe export requires exactly one entry",
                    ));
                }
                ("naiv4vibe", entries[0].official_document.clone())
            }
            VibeExportFormat::Naiv4vibebundle => (
                "naiv4vibebundle",
                json!({
                    "identifier": OFFICIAL_VIBE_BUNDLE_IDENTIFIER,
                    "version": 1,
                    "vibes": entries
                        .iter()
                        .map(|entry| entry.official_document.clone())
                        .collect::<Vec<_>>(),
                }),
            ),
        };
        let content = serde_json::to_string_pretty(&value)
            .map_err(|error| VibeError::invalid_document(error.to_string()))?;
        Ok(VibeExportDocument {
            file_extension,
            content,
        })
    }
}

fn import_bundle(
    root: &Map<String, Value>,
    file_name: &str,
) -> VibeDomainResult<Vec<VibeImportEntry>> {
    let vibes = root
        .get("vibes")
        .and_then(Value::as_array)
        .ok_or_else(|| VibeError::invalid_document("vibe bundle is missing vibes array"))?;
    if vibes.is_empty() {
        return Err(VibeError::invalid_document("vibe bundle cannot be empty"));
    }
    vibes
        .iter()
        .enumerate()
        .map(|(index, item)| validate_official_vibe_object(item, &format!("{file_name}#{index}")))
        .collect()
}

fn validate_official_vibe_object(
    value: &Value,
    fallback_name: &str,
) -> VibeDomainResult<VibeImportEntry> {
    let root = value
        .as_object()
        .ok_or_else(|| VibeError::invalid_document("vibe entry must be an object"))?;
    if root.get("identifier").and_then(Value::as_str) != Some(OFFICIAL_VIBE_IDENTIFIER) {
        return Err(VibeError::invalid_document("invalid .naiv4vibe identifier"));
    }
    let vibe_type = root
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| VibeError::invalid_document("missing .naiv4vibe type"))?;
    if vibe_type != "image" && vibe_type != "encoding" {
        return Err(VibeError::invalid_document(
            ".naiv4vibe type must be image or encoding",
        ));
    }
    let id = root
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| VibeError::invalid_document("missing .naiv4vibe id"))?;
    if !is_sha256_hex(id) {
        return Err(VibeError::invalid_document(
            ".naiv4vibe id must be a 64-character sha256 hex string",
        ));
    }
    let preview_payload = validate_thumbnail(root)?;
    let encodings = root
        .get("encodings")
        .and_then(Value::as_object)
        .ok_or_else(|| VibeError::invalid_document("missing .naiv4vibe encodings"))?;
    let first_encoding = first_encoding_payload(encodings)
        .ok_or_else(|| VibeError::invalid_document("missing .naiv4vibe encoding payload"))?;
    validate_payload_identity(root, vibe_type, id, first_encoding)?;
    let display_name = root
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| fallback_display_name(fallback_name), ToOwned::to_owned);
    let available_model_keys = encodings.keys().cloned().collect::<Vec<_>>();
    let encoding_payloads = collect_encoding_payloads(encodings)?;
    let available_encoding_configs = encoding_payloads
        .iter()
        .filter_map(|encoding| encoding.config.clone())
        .collect();
    let official_document = value.clone();
    let document_payload = serde_json::to_string(&official_document)
        .map_err(|error| VibeError::invalid_document(error.to_string()))?;
    let source_image_payload = root
        .get("image")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    Ok(VibeImportEntry {
        summary: VibeDocumentSummary {
            document_id: VibeId::new(id),
            display_name,
            has_image: vibe_type == "image",
            hidden: false,
            available_model_keys,
            available_encoding_configs,
            created_at_ms: 0,
            updated_at_ms: 0,
        },
        official_document,
        document_payload,
        source_image_payload,
        preview_payload,
        encoding_payloads,
    })
}

fn collect_encoding_payloads(
    encodings: &Map<String, Value>,
) -> VibeDomainResult<Vec<VibeImportedEncoding>> {
    let mut payloads = Vec::new();
    for (model_key, bucket) in encodings {
        let model = VibeModel::from_vibe_model_key(model_key);
        let Some(entries) = bucket.as_object() else {
            continue;
        };
        for (encoding_key, entry) in entries {
            let payload = entry.as_str().map_or_else(
                || {
                    entry
                        .as_object()
                        .and_then(|object| object.get("encoding"))
                        .and_then(Value::as_str)
                },
                Some,
            );
            let Some(payload) = payload.filter(|value| !value.trim().is_empty()) else {
                continue;
            };
            STANDARD
                .decode(payload)
                .map_err(|error| VibeError::invalid_document(error.to_string()))?;
            let config = model.and_then(|model| {
                let information_extracted = entry
                    .as_object()
                    .and_then(|object| object.get("params"))
                    .and_then(Value::as_object)
                    .and_then(|params| params.get("information_extracted"))
                    .and_then(json_number_as_f32)
                    .unwrap_or(0.7);
                VibeEncodeSettings::new(model, information_extracted)
                    .ok()
                    .map(|settings| VibeEncodingConfig { model, settings })
            });
            payloads.push(VibeImportedEncoding {
                model_key: model_key.clone(),
                encoding_key: encoding_key.clone(),
                payload: payload.to_owned(),
                config,
            });
        }
    }
    Ok(payloads)
}

fn validate_payload_identity(
    root: &Map<String, Value>,
    vibe_type: &str,
    id: &str,
    first_encoding: &str,
) -> VibeDomainResult<()> {
    match vibe_type {
        "image" => {
            let image = root
                .get("image")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| VibeError::invalid_document("missing .naiv4vibe image"))?;
            validate_base64_or_data_url(image, "image")?;
            if sha256_hex(image.as_bytes()) != id {
                return Err(VibeError::invalid_document(
                    ".naiv4vibe id does not match image payload hash",
                ));
            }
        }
        "encoding" => {
            STANDARD
                .decode(first_encoding)
                .map_err(|error| VibeError::invalid_document(error.to_string()))?;
            if sha256_hex(first_encoding.as_bytes()) != id {
                return Err(VibeError::invalid_document(
                    ".naiv4vibe id does not match encoding payload hash",
                ));
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn validate_thumbnail(root: &Map<String, Value>) -> VibeDomainResult<Option<String>> {
    if let Some(thumbnail) = root
        .get("thumbnail")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !thumbnail.starts_with("data:image/") || !thumbnail.contains(";base64,") {
            return Err(VibeError::invalid_document(
                ".naiv4vibe thumbnail must be an image data URL",
            ));
        }
        validate_base64_or_data_url(thumbnail, "thumbnail")?;
        return Ok(Some(thumbnail.to_owned()));
    }
    Ok(None)
}

fn first_encoding_payload(encodings: &Map<String, Value>) -> Option<&str> {
    for bucket in encodings.values() {
        let entries = bucket.as_object()?;
        for entry in entries.values() {
            if let Some(value) = entry.as_str().filter(|value| !value.trim().is_empty()) {
                return Some(value);
            }
            if let Some(value) = entry
                .as_object()
                .and_then(|object| object.get("encoding"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                return Some(value);
            }
        }
    }
    None
}

fn json_number_as_f32(value: &Value) -> Option<f32> {
    value
        .as_number()
        .and_then(|_| value.to_string().parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

fn validate_base64_or_data_url(value: &str, field: &str) -> VibeDomainResult<()> {
    let payload = value
        .split_once(";base64,")
        .map_or(value, |(_, payload)| payload);
    STANDARD
        .decode(payload)
        .map(|_| ())
        .map_err(|error| VibeError::invalid_document(format!("{field} is not base64: {error}")))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing a SHA-256 digest to String cannot fail");
    }
    output
}

fn fallback_display_name(file_name: &str) -> String {
    file_name
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.split('.').next())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("vibe")
        .to_owned()
}
