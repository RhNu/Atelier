use serde_json::{Value, json};

const TO_VERSION: u32 = 4;

pub(super) fn migrate(mut value: Value) -> Result<(Value, u32), String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| "global settings v3 payload is not an object".to_owned())?;
    let frontend = object
        .get_mut("frontend")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "global settings v3 frontend is not an object".to_owned())?;
    frontend
        .entry("convert_full_width_punctuation".to_owned())
        .or_insert_with(|| json!(false));
    object.insert("schema_version".to_owned(), Value::from(TO_VERSION));
    Ok((value, TO_VERSION))
}
