use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentResult};
use atelier_app_api::{
    generation::ImageModelDto,
    prompt::{PromptPresetBehaviorDto, PromptPresetKindDto},
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    resource_edit::ResourceKind,
    tools::{AgentTools, agent_app_error, parse_args},
};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Search {
    query: String,
    kind: Option<ResourceKind>,
    preset_kind: Option<PromptPresetKindDto>,
    model: Option<ImageModelDto>,
    offset: usize,
    limit: Option<usize>,
}

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn search_resources(&self, arguments: &str) -> AgentResult<String> {
        let args: Search = parse_args(arguments)?;
        let limit = args.limit.unwrap_or(30);
        if !(1..=100).contains(&limit) {
            return Err(AgentError::validation("limit must be between 1 and 100"));
        }
        let model = args.model.map(crate::mapping::image_model_to_domain);
        let query = args.query.trim().to_lowercase();
        let mut matches = Vec::new();
        let _guard = self.app.prompt_resource_write.lock().await;
        if args.kind != Some(ResourceKind::Preset) && args.preset_kind.is_none() {
            for value in self
                .app
                .prompt_chunks
                .list_chunks(model)
                .await
                .map_err(|error| AgentError::runtime(error.to_string()))?
            {
                if search_matches(
                    &query,
                    &[
                        value.key.as_str(),
                        &value.display_name,
                        &value.aliases.join(" "),
                        &value.content,
                        value.description.as_deref().unwrap_or_default(),
                    ],
                ) {
                    matches.push(json!({"kind":"chunk","id":value.id.as_str(),"path":value.key.as_str(),"display_name":value.display_name,"aliases":value.aliases,"description":value.description,"models":value.models.into_iter().map(crate::mapping::image_model_to_dto).collect::<Vec<_>>()}));
                }
            }
        }
        if args.kind != Some(ResourceKind::Chunk) {
            for value in self
                .app
                .prompt_presets
                .list_presets(
                    args.preset_kind
                        .map(crate::mapping::prompt_preset_kind_to_domain),
                    model,
                )
                .await
                .map_err(|error| AgentError::runtime(error.to_string()))?
            {
                let dto = crate::mapping::prompt_preset_to_dto(&value);
                let prompt_fields = behavior_fields(&dto.prompt_behavior);
                let uc_fields = behavior_fields(&dto.uc_behavior);
                if search_matches(
                    &query,
                    &[
                        &dto.path,
                        &dto.display_name,
                        &dto.aliases.join(" "),
                        prompt_fields[0],
                        prompt_fields[1],
                        uc_fields[0],
                        uc_fields[1],
                        dto.description.as_deref().unwrap_or_default(),
                    ],
                ) {
                    matches.push(json!({"kind":"preset","id":dto.preset_id,"preset_kind":dto.kind,"path":dto.path,"display_name":dto.display_name,"aliases":dto.aliases,"description":dto.description,"models":dto.models}));
                }
            }
        }
        matches.sort_by(|a, b| {
            a["path"]
                .as_str()
                .cmp(&b["path"].as_str())
                .then(a["id"].as_str().cmp(&b["id"].as_str()))
        });
        let total = matches.len();
        let items = matches
            .into_iter()
            .skip(args.offset)
            .take(limit)
            .collect::<Vec<Value>>();
        Ok(json!({"items":items,"total":total,"offset":args.offset,"limit":limit}).to_string())
    }

    pub(super) async fn get_prompt_library(&self) -> AgentResult<String> {
        use atelier_app_api::resource_library::{
            GetLibrarySnapshotRequestDto, LibraryNamespaceDto,
        };
        let _guard = self.app.prompt_resource_write.lock().await;
        let mut libraries = Vec::new();
        for namespace in [
            LibraryNamespaceDto::PromptChunk,
            LibraryNamespaceDto::MainPreset,
            LibraryNamespaceDto::CharacterPreset,
        ] {
            let snapshot = self
                .app
                .resource_library()
                .snapshot(GetLibrarySnapshotRequestDto { namespace })
                .await
                .map_err(agent_app_error)?;
            libraries.push(json!({"namespace":namespace,"folders":snapshot.folders}));
        }
        Ok(json!({"libraries":libraries}).to_string())
    }
}

fn behavior_fields(value: &PromptPresetBehaviorDto) -> [&str; 2] {
    match value {
        PromptPresetBehaviorDto::Surround { before, after } => [before, after],
        PromptPresetBehaviorDto::Replace { text } => [text, ""],
    }
}

fn search_matches(query: &str, fields: &[&str]) -> bool {
    query.is_empty()
        || fields
            .iter()
            .any(|field| field.to_lowercase().contains(query))
}

#[cfg(test)]
mod tests {
    use super::{PromptPresetBehaviorDto, behavior_fields, search_matches};
    #[test]
    fn searches_all_text_fields_case_insensitively() {
        assert!(search_matches("fox", &["portrait", "Fox ears"]));
        assert!(!search_matches("fox", &["forest", "portrait"]));
        assert!(search_matches("", &[]));
    }
    #[test]
    fn preset_search_uses_literal_text_with_quotes_and_newlines() {
        let behavior = PromptPresetBehaviorDto::Replace {
            text: r#"a sign reading "hello"
forest"#
                .to_owned(),
        };
        assert!(search_matches(
            "\"hello\"\nforest",
            &behavior_fields(&behavior)
        ));
    }
}
