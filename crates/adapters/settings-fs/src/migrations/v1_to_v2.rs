use serde_json::{Value, json};

const TO_VERSION: u32 = 2;

pub(super) fn migrate(mut value: Value) -> Result<(Value, u32), String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| "global settings v1 payload is not an object".to_owned())?;
    object.insert("schema_version".to_owned(), Value::from(TO_VERSION));
    object.insert(
        "safety".to_owned(),
        json!({ "wd_auto_review_enabled": false }),
    );
    Ok((value, TO_VERSION))
}
