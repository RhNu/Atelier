use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct PipelineEntity {
    pub id: u64,
    pub canonical_name: String,
    pub primary_translation: String,
    pub kind: String,
    pub category: String,
    pub post_count: u64,
    pub rating: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub translations: Vec<LocalizedText>,
    #[serde(default)]
    pub wiki: Vec<LocalizedText>,
    #[serde(default)]
    pub expansion_terms: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LocalizedText {
    pub locale: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PipelineGroup {
    pub id: String,
    pub name: String,
    pub members: Vec<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PipelineRelation {
    pub source_entity_id: u64,
    pub target_entity_id: u64,
    pub relation: String,
    pub npmi: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PipelineProvenance {
    #[serde(default)]
    pub sources: Vec<PipelineSource>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PipelineSource {
    pub id: String,
    pub url: String,
    pub snapshot: String,
    pub sha256: String,
    pub license: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SemanticConfig {
    pub dimensions: usize,
    pub entity_count: usize,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
    #[serde(default = "default_query_prefix")]
    pub query_prefix: String,
    #[serde(default = "default_passage_prefix")]
    pub passage_prefix: String,
    #[serde(default = "default_input_ids")]
    pub input_ids: String,
    #[serde(default = "default_attention_mask")]
    pub attention_mask: String,
    #[serde(default = "default_token_type_ids")]
    pub token_type_ids: Option<String>,
    #[serde(default)]
    pub output_name: Option<String>,
}

const fn default_max_length() -> usize {
    512
}

fn default_query_prefix() -> String {
    "query: ".to_owned()
}

fn default_passage_prefix() -> String {
    "passage: ".to_owned()
}

fn default_input_ids() -> String {
    "input_ids".to_owned()
}

fn default_attention_mask() -> String {
    "attention_mask".to_owned()
}

#[allow(clippy::unnecessary_wraps)]
fn default_token_type_ids() -> Option<String> {
    Some("token_type_ids".to_owned())
}
