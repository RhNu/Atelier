use atelier_agent::AgentToolSpec;
use atelier_app_api::generation::{
    ImageFormatDto, NoiseScheduleDto, QualityPresetDto, SamplerDto, UcPresetDto,
};
use serde_json::{Value, json};

pub fn tool_specs() -> Vec<AgentToolSpec> {
    let mut specs = vec![
        spec(
            "get_generation_context",
            "Read the current draft and model capabilities. Versions are managed by the host.",
            object(json!({}), &[]),
        ),
        spec(
            "edit_generation_draft",
            "Apply ordered draft operations atomically. Use one call for all edits to this draft in a model response. Omitted patch fields stay unchanged. Text replacement is exact and unique unless all=true. After outdated, review the returned draft and replan in the next response.",
            object(
                json!({"operations":{"type":"array","minItems":1,"items":{"oneOf":draft_operations()}}}),
                &["operations"],
            ),
        ),
        spec(
            "preview_generation",
            "Compile the observed text-only draft with presets and chunks. Read expanded text, effective overrides, token usage and optional Anlas estimate. Review this result in the next model response before submission; the host binds submission to the compiled snapshot.",
            object(json!({}), &[]),
        ),
        spec(
            "submit_generation",
            "Submit the previously observed compiled preview as a generation batch. Requires preview_generation first. Changed draft or prompt resources invalidate the preview.",
            object(json!({}), &[]),
        ),
        spec(
            "undo_agent_action",
            "Undo a draft edit by action_id if no subsequent edit occurred.",
            object(json!({"action_id":{"type":"string"}}), &["action_id"]),
        ),
    ];
    specs.extend(super::resource_schemas::specs());
    specs.extend(super::lexicon::specs());
    specs.extend(super::generation_feedback::specs());
    specs.push(super::output_vision::spec());
    specs
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "schemas are short-lived values serialized at this boundary"
)]
pub(super) fn spec(name: &str, description: &str, schema: Value) -> AgentToolSpec {
    AgentToolSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters_json: schema.to_string(),
    }
}

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({"type":"object","required":required,"additionalProperties":false});
    schema["properties"] = properties;
    schema
}

pub(super) fn text_edit_schema() -> Value {
    json!({"oneOf":[
        object(json!({"mode":{"const":"set"},"text":{"type":"string"}}), &["mode","text"]),
        object(json!({"mode":{"const":"append"},"text":{"type":"string"}}), &["mode","text"]),
        object(json!({"mode":{"const":"replace"},"old_text":{"type":"string","minLength":1},"new_text":{"type":"string"},"all":{"type":"boolean"}}), &["mode","old_text","new_text"])
    ]})
}

pub(super) fn operation(name: &str, mut properties: Value, required: &[&str]) -> Value {
    properties["op"] = json!({"const":name});
    let mut fields = vec!["op"];
    fields.extend_from_slice(required);
    object(properties, &fields)
}

fn draft_operations() -> Vec<Value> {
    vec![
        operation(
            "edit_text",
            json!({"target":object(json!({"character_id":{"type":"string"},"field":{"enum":["prompt","negative_prompt"]}}), &["field"]),"edit":text_edit_schema()}),
            &["target", "edit"],
        ),
        operation(
            "create_character",
            json!({"id":{"type":"string","description":"New stable character id, unique within this model."},"prompt":{"type":"string"}}),
            &["id", "prompt"],
        ),
        operation(
            "update_character",
            json!({"id":{"type":"string"},"patch":object(json!({"enabled":{"type":"boolean"},"position":object(json!({"x":{"type":"number","minimum":0,"maximum":1},"y":{"type":"number","minimum":0,"maximum":1}}), &["x","y"])}), &[])}),
            &["id", "patch"],
        ),
        operation(
            "copy_character",
            json!({"id":{"type":"string"},"new_id":{"type":"string"}}),
            &["id", "new_id"],
        ),
        operation("remove_character", json!({"id":{"type":"string"}}), &["id"]),
        operation(
            "reorder_characters",
            json!({"ids":{"type":"array","items":{"type":"string"},"uniqueItems":true}}),
            &["ids"],
        ),
        operation(
            "select_preset",
            json!({"character_id":{"type":"string"},"preset_id":{"type":["string","null"],"description":"Preset id, or null to deselect. Omit character_id for the main preset."}}),
            &["preset_id"],
        ),
        operation(
            "set_parameters",
            json!({"patch":parameter_schema()}),
            &["patch"],
        ),
        operation(
            "select_model",
            json!({"model":{"type":"string","description":"Exact model identifier from get_generation_context. Restores its prompt state and resets scale to its default."}}),
            &["model"],
        ),
    ]
}

fn parameter_schema() -> Value {
    let mut properties = json!({
        "size":object(json!({"width":{"type":"integer"},"height":{"type":"integer"}}), &["width","height"]),
        "seed_mode":{"enum":["random","fixed"]},
        "character_position_mode":{"enum":["global","manual"]}
    });
    for field in ["steps", "seed", "n_samples", "request_count"] {
        properties[field] = json!({"type":"integer"});
    }
    for field in ["scale", "cfg_rescale"] {
        properties[field] = json!({"type":"number"});
    }
    for field in [
        "variety_boost",
        "strict_mode",
        "stream_enabled",
        "transparent_background",
        "furry_mode",
    ] {
        properties[field] = json!({"type":"boolean"});
    }
    properties["quality"] = json!({"enum":[QualityPresetDto::Standard, QualityPresetDto::Light, QualityPresetDto::None]});
    properties["uc_preset"] = json!({"enum":[UcPresetDto::Heavy, UcPresetDto::Light, UcPresetDto::FurryFocus, UcPresetDto::HumanFocus, UcPresetDto::None]});
    properties["sampler"] = json!({"enum":[SamplerDto::KEuler, SamplerDto::KEulerAncestral, SamplerDto::KDpm2, SamplerDto::KDpm2Ancestral, SamplerDto::KDpmpp2m, SamplerDto::KDpmpp2mSde, SamplerDto::KDpmpp2sAncestral, SamplerDto::KDpmppSde, SamplerDto::Ddim, SamplerDto::DdimV3]});
    properties["noise_schedule"] = json!({"enum":[NoiseScheduleDto::Native, NoiseScheduleDto::Karras, NoiseScheduleDto::Exponential, NoiseScheduleDto::Polyexponential]});
    properties["image_format"] = json!({"enum":[ImageFormatDto::Png, ImageFormatDto::Webp, null]});
    object(properties, &[])
}
