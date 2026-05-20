use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRefDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
}
