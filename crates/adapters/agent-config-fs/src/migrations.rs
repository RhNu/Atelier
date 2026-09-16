use atelier_agent::{AgentError, AgentResult};
use serde_json::{Value, json};

const CURRENT_VERSION: u64 = 2;

pub fn upgrade(mut value: Value) -> AgentResult<Value> {
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| repository_error("Agent registry schema version is missing"))?;
    match version {
        1 => migrate_v1_to_v2(&mut value)?,
        CURRENT_VERSION => {}
        _ => {
            return Err(repository_error(format!(
                "unsupported Agent registry schema version {version}"
            )));
        }
    }
    Ok(value)
}

fn migrate_v1_to_v2(value: &mut Value) -> AgentResult<()> {
    let root = value
        .as_object_mut()
        .ok_or_else(|| repository_error("Agent registry root must be an object"))?;
    let connections = root
        .get_mut("connections")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| repository_error("Agent registry connections must be an array"))?;
    for connection in connections {
        connection
            .as_object_mut()
            .ok_or_else(|| repository_error("Agent registry connection must be an object"))?
            .insert("protocol".to_owned(), Value::from("chat_completions"));
    }
    let models = root
        .get_mut("models")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| repository_error("Agent registry models must be an array"))?;
    for model in models {
        let model = model
            .as_object_mut()
            .ok_or_else(|| repository_error("Agent registry model must be an object"))?;
        let image_input = if model
            .remove("supports_vision")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            "tool_result"
        } else {
            "none"
        };
        model.insert(
            "capabilities".to_owned(),
            json!({ "image_input": image_input }),
        );
    }
    root.insert("schema_version".to_owned(), Value::from(CURRENT_VERSION));
    Ok(())
}

fn repository_error(message: impl Into<String>) -> AgentError {
    AgentError::repository(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_one_fields_are_upgraded_deterministically() {
        let upgraded = upgrade(json!({
            "format": "atelier-agent-registry",
            "schema_version": 1,
            "connections": [{"id":"c"}],
            "models": [
                {"id":"vision", "supports_vision":true},
                {"id":"text", "supports_vision":false}
            ]
        }))
        .expect("upgrade");

        assert_eq!(upgraded["schema_version"], 2);
        assert_eq!(upgraded["connections"][0]["protocol"], "chat_completions");
        assert_eq!(
            upgraded["models"][0]["capabilities"]["image_input"],
            "tool_result"
        );
        assert_eq!(upgraded["models"][1]["capabilities"]["image_input"], "none");
    }
}
