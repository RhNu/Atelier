use atelier_explore::novelai::{
    ExploreCharacterCaption, ExploreCharacterCenter, ExploreGenerationParameter,
    ExploreMetadataStatus, NovelAiExploreMetadata,
};
use serde_json::Value;

const MAX_METADATA_BYTES: usize = 256 * 1024;

pub fn parse_metadata(raw: Option<&Value>) -> NovelAiExploreMetadata {
    let raw = raw
        .filter(|v| !v.is_null())
        .map(|v| match v {
            Value::String(text) => text.clone(),
            value => value.to_string(),
        })
        .filter(|text| !text.trim().is_empty());
    let mut result = NovelAiExploreMetadata {
        status: ExploreMetadataStatus::Missing,
        prompt: None,
        negative_prompt: None,
        characters: vec![],
        negative_characters: vec![],
        use_coords: None,
        use_order: None,
        negative_use_coords: None,
        negative_use_order: None,
        parameters: vec![],
        raw: None,
        warnings: vec![],
    };
    let Some(raw) = raw else {
        return result;
    };
    if raw.len() > MAX_METADATA_BYTES {
        result.status = ExploreMetadataStatus::Invalid;
        result
            .warnings
            .push("metadata exceeds the supported size".into());
        return result;
    }
    result.raw = Some(raw.clone());
    let Ok(Value::Object(envelope)) = serde_json::from_str::<Value>(&raw) else {
        result.status = ExploreMetadataStatus::Invalid;
        result.warnings.push("metadata is not a JSON object".into());
        return result;
    };
    result.prompt = text(envelope.get("Description"));
    let comment = match envelope.get("Comment") {
        Some(Value::String(value)) => serde_json::from_str::<Value>(value).ok(),
        Some(Value::Object(object)) => Some(Value::Object(object.clone())),
        None => Some(Value::Object(envelope.clone())),
        _ => None,
    };
    let Some(Value::Object(comment)) = comment else {
        result.status = ExploreMetadataStatus::Partial;
        result
            .warnings
            .push("Comment metadata could not be decoded".into());
        return result;
    };
    fill_comment(&mut result, &comment, &envelope);
    result.status = if !result.warnings.is_empty() {
        ExploreMetadataStatus::Partial
    } else if result.prompt.is_none()
        && result.negative_prompt.is_none()
        && result.characters.is_empty()
        && result.negative_characters.is_empty()
        && result.parameters.is_empty()
    {
        ExploreMetadataStatus::Missing
    } else {
        ExploreMetadataStatus::Available
    };
    result
}

fn fill_comment(
    result: &mut NovelAiExploreMetadata,
    comment: &serde_json::Map<String, Value>,
    envelope: &serde_json::Map<String, Value>,
) {
    let positive = comment.get("v4_prompt");
    let negative = comment.get("v4_negative_prompt");
    result.prompt = text(positive.and_then(|v| v.pointer("/caption/base_caption")))
        .or_else(|| text(comment.get("prompt")))
        .or_else(|| result.prompt.take());
    result.negative_prompt = text(negative.and_then(|v| v.pointer("/caption/base_caption")))
        .or_else(|| text(comment.get("uc")))
        .or_else(|| text(comment.get("negative_prompt")));
    result.characters = captions(positive, &mut result.warnings);
    result.negative_characters = captions(negative, &mut result.warnings);
    result.use_coords = positive
        .and_then(|v| v.get("use_coords"))
        .and_then(Value::as_bool);
    result.use_order = positive
        .and_then(|v| v.get("use_order"))
        .and_then(Value::as_bool);
    result.negative_use_coords = negative
        .and_then(|v| v.get("use_coords"))
        .and_then(Value::as_bool);
    result.negative_use_order = negative
        .and_then(|v| v.get("use_order"))
        .and_then(Value::as_bool);
    for name in [
        "model_name",
        "model",
        "seed",
        "steps",
        "sampler",
        "width",
        "height",
        "scale",
        "noise_schedule",
        "cfg_rescale",
        "quality_boost",
        "tag_hint_uc_preset",
        "sm",
        "sm_dyn",
    ] {
        if let Some(value) = comment
            .get(name)
            .filter(|v| v.is_string() || v.is_number() || v.is_boolean())
        {
            result.parameters.push(ExploreGenerationParameter {
                name: name.into(),
                value: value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned),
            });
        }
    }
    if !result
        .parameters
        .iter()
        .any(|p| p.name == "model_name" || p.name == "model")
        && let Some(source) = text(envelope.get("Source"))
    {
        result.parameters.push(ExploreGenerationParameter {
            name: "model_name".into(),
            value: source,
        });
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}

fn captions(value: Option<&Value>, warnings: &mut Vec<String>) -> Vec<ExploreCharacterCaption> {
    let Some(raw) = value.and_then(|v| v.pointer("/caption/char_captions")) else {
        return vec![];
    };
    let Some(items) = raw.as_array() else {
        warnings.push("character captions are not an array".into());
        return vec![];
    };
    if items.len() > 64 {
        warnings.push(
            "character caption display truncated to 64 entries; raw metadata retained".into(),
        );
    }
    items
        .iter()
        .take(64)
        .map(|item| {
            let text = text(item.get("char_caption")).unwrap_or_else(|| {
                warnings.push("a character caption has no text; its position is retained".into());
                String::new()
            });
            let centers = item
                .get("centers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(64)
                .filter_map(|center| {
                    let point = center
                        .get("x")
                        .and_then(Value::as_f64)
                        .zip(center.get("y").and_then(Value::as_f64));
                    if let Some((x, y)) = point.filter(|(x, y)| x.is_finite() && y.is_finite()) {
                        Some(ExploreCharacterCenter { x, y })
                    } else {
                        warnings.push("a character coordinate could not be decoded".into());
                        None
                    }
                })
                .collect();
            ExploreCharacterCaption { text, centers }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_strings_preserve_prompt_syntax_and_separate_character_arrays() {
        let raw = json!(json!({"Comment": json!({
            "prompt":"2::blue sky::, {cloud}", "uc":"noise", "seed":0,
            "model_name":"Future model",
            "v4_prompt":{"caption":{"base_caption":"  2::blue sky::, {cloud}  ","char_captions":[
                {"char_caption":"first","centers":[{"x":0.2,"y":0.7}]}, {"char_caption":""}]},"use_coords":true},
            "v4_negative_prompt":{"caption":{"char_captions":[{"char_caption":"blur"}]}}
        }).to_string()}).to_string());
        let parsed = parse_metadata(Some(&raw));
        assert_eq!(parsed.status, ExploreMetadataStatus::Available);
        assert_eq!(parsed.prompt.as_deref(), Some("  2::blue sky::, {cloud}  "));
        assert_eq!(parsed.characters.len(), 2);
        assert_eq!(parsed.negative_characters.len(), 1);
        assert!((parsed.characters[0].centers[0].x - 0.2).abs() < f64::EPSILON);
        assert_eq!(
            parsed
                .parameters
                .iter()
                .find(|p| p.name == "seed")
                .unwrap()
                .value,
            "0"
        );
    }

    #[test]
    fn damaged_comment_retains_description_and_raw_data() {
        let raw = json!({"Description":"fallback", "Comment":"{"});
        let parsed = parse_metadata(Some(&raw));
        assert_eq!(parsed.status, ExploreMetadataStatus::Partial);
        assert_eq!(parsed.prompt.as_deref(), Some("fallback"));
        assert!(parsed.raw.is_some());
        assert_eq!(parse_metadata(None).status, ExploreMetadataStatus::Missing);
        assert_eq!(
            parse_metadata(Some(&json!("bad"))).status,
            ExploreMetadataStatus::Invalid
        );
    }
}
