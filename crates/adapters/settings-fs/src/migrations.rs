use serde_json::Value;

mod v1_to_v2;
mod v2_to_v3;

const FORMAT: &str = "atelier-global-settings";
const CURRENT_VERSION: u32 = 3;

pub struct MigrationResult {
    pub text: String,
    pub changed: bool,
}

pub fn migrate(text: &str) -> Result<MigrationResult, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let format = value
        .get("format")
        .and_then(Value::as_str)
        .ok_or_else(|| "global settings format is missing".to_owned())?;
    let mut version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "global settings schema version is missing or invalid".to_owned())?;
    if format != FORMAT {
        return Err(format!("unsupported global settings format `{format}`"));
    }
    if version > CURRENT_VERSION {
        return Err(format!(
            "global settings schema version {version} is newer than supported version \
             {CURRENT_VERSION}"
        ));
    }

    let original_version = version;
    while version < CURRENT_VERSION {
        (value, version) = match version {
            1 => v1_to_v2::migrate(value)?,
            2 => v2_to_v3::migrate(value)?,
            unsupported => {
                return Err(format!(
                    "no global settings migration starts at schema version {unsupported}"
                ));
            }
        };
    }
    Ok(MigrationResult {
        text: serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?,
        changed: original_version != version,
    })
}
