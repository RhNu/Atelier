use atelier_agent::AgentToolSpec;
use atelier_app_api::generation::{QualityPresetDto, UcPresetDto};
use serde_json::{Value, json};

use super::schemas::{object, operation, spec, text_edit_schema};

pub fn specs() -> Vec<AgentToolSpec> {
    vec![
        spec(
            "search_prompt_resources",
            "Search ALL prompt resources before pagination. Omit model to search across models. Results are summaries; read a resource before editing, copying or deleting it.",
            object(
                json!({
                    "query":{"type":"string"},"kind":{"enum":["chunk","preset"]},"preset_kind":{"enum":["main","character"]},
                    "model":{"type":"string"},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":100}
                }),
                &[],
            ),
        ),
        spec(
            "get_prompt_resource",
            "Read complete chunk or main/character preset content, metadata and inbound dependencies. The host remembers the observation; no revision argument is needed.",
            target(),
        ),
        spec(
            "get_prompt_library",
            "Read prompt library folders for assigning resource paths and folder_id. Paths are library paths, not filesystem paths.",
            object(json!({}), &[]),
        ),
        spec(
            "create_prompt_resource",
            "Create a text chunk or a complete main/character preset. Preview images are outside this tool. Use a valid existing folder_id for nested paths.",
            json!({"type":"object","oneOf":[
                object(json!({"kind":{"const":"chunk"},"metadata":metadata(false),"content":{"type":"string"}}), &["kind","metadata","content"]),
                object(json!({"kind":{"const":"preset"},"metadata":metadata(false),"preset_kind":{"enum":["main","character"]},"prompt_behavior":behavior(),"uc_behavior":behavior(),"quality_override":quality(),"uc_preset_override":uc_preset()}), &["kind","metadata","preset_kind","prompt_behavior","uc_behavior"])
            ]}),
        ),
        spec(
            "edit_prompt_resource",
            "Atomically apply ordered operations to an observed resource. Omitted fields are preserved; null clears optional fields. Exact text replacement must be unique unless all=true. A path change rewrites chunk references. On outdated review the returned resource and replan; never replay queued edits blindly.",
            object(
                json!({"target":target(),"operations":{"type":"array","minItems":1,"items":{"oneOf":[
                    operation("edit_text",json!({"field":{"enum":["content","prompt_before","prompt_after","prompt_text","uc_before","uc_after","uc_text"]},"edit":text_edit_schema()}), &["field","edit"]),
                    operation("set_metadata",json!({"patch":metadata(true)}), &["patch"]),
                    operation("set_behavior",json!({"field":{"enum":["prompt","negative_prompt"]},"behavior":behavior()}), &["field","behavior"]),
                    operation("set_overrides",json!({"patch":object(json!({"quality":quality(),"uc_preset":uc_preset()}), &[])}), &["patch"])
                ]}}}),
                &["target", "operations"],
            ),
        ),
        spec(
            "copy_prompt_resource",
            "Copy an observed resource to a new library path, preserving its text, preset overrides, model bindings and existing preview. Omit folder_id for the root.",
            object(
                json!({"target":target(),"path":{"type":"string"},"folder_id":{"type":["string","null"]},"display_name":{"type":"string"}}),
                &["target", "path", "display_name"],
            ),
        ),
        spec(
            "delete_prompt_resource",
            "Delete an observed, unreferenced prompt resource. Refuses removal while any saved draft or prompt resource uses it.",
            target(),
        ),
    ]
}

fn target() -> Value {
    object(
        json!({"kind":{"enum":["chunk","preset"]},"id":{"type":"string"}}),
        &["kind", "id"],
    )
}

fn metadata(patch: bool) -> Value {
    object(
        json!({
            "path":{"type":"string"},"folder_id":{"type":["string","null"]},"display_name":{"type":"string"},
            "aliases":{"type":"array","items":{"type":"string"}},"description":{"type":["string","null"]},
            "models":{"type":"array","minItems":1,"uniqueItems":true,"items":{"type":"string","description":"Model identifier from get_generation_context."}}
        }),
        if patch {
            &[]
        } else {
            &["path", "display_name", "models"]
        },
    )
}

fn behavior() -> Value {
    json!({"oneOf":[
        object(json!({"mode":{"const":"surround"},"before":{"type":"string"},"after":{"type":"string"}}), &["mode","before","after"]),
        object(json!({"mode":{"const":"replace"},"text":{"type":"string"}}), &["mode","text"])
    ]})
}

fn quality() -> Value {
    json!({"enum":[QualityPresetDto::Standard, QualityPresetDto::Light, QualityPresetDto::None, null]})
}

fn uc_preset() -> Value {
    json!({"enum":[UcPresetDto::Heavy,UcPresetDto::Light,UcPresetDto::FurryFocus,UcPresetDto::HumanFocus,UcPresetDto::None,null]})
}
