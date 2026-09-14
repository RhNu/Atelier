use std::collections::BTreeMap;

use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentObservation, AgentResult};
use atelier_app_api::prompt::{DeletePromptChunkRequestDto, DeletePromptPresetRequestDto};
use atelier_prompt_resources::{PromptChunkId, PromptPresetId};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::json;

use super::{
    resource_edit::{ResourceDocument, ResourceKind, ResourceOperation, ResourceTarget},
    resource_input::{CreateResource, ResourceWrite},
    tools::{AgentTools, agent_app_error, parse_args},
};

pub type ResourceObservations =
    BTreeMap<ResourceTarget, AgentObservation<Option<ResourceDocument>>>;

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) fn resources_observed(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, ResourceObservations>> {
        self.resource_observations
            .lock()
            .map_err(|_| AgentError::runtime("resource observations unavailable"))
    }

    pub(super) async fn read_resource(
        &self,
        target: &ResourceTarget,
    ) -> AgentResult<Option<ResourceDocument>> {
        match target.kind {
            ResourceKind::Chunk => self
                .app
                .prompt_chunks
                .get_chunk_by_id(&PromptChunkId::new(&target.id))
                .await
                .map(|value| {
                    value.map(|value| {
                        ResourceDocument::Chunk(crate::mapping::prompt_chunk_to_dto(&value))
                    })
                }),
            ResourceKind::Preset => self
                .app
                .prompt_presets
                .get_preset_by_id(&PromptPresetId::new(&target.id))
                .await
                .map(|value| {
                    value.map(|value| {
                        ResourceDocument::Preset(crate::mapping::prompt_preset_to_dto(&value))
                    })
                }),
        }
        .map_err(|error| AgentError::runtime(error.to_string()))
    }

    pub(super) async fn get_resource(&self, arguments: &str) -> AgentResult<String> {
        let target: ResourceTarget = parse_args(arguments)?;
        let _guard = self.app.generation_draft_write.lock().await;
        let _resources = self.app.prompt_resource_write.lock().await;
        let current = self.read_resource(&target).await?;
        let dependencies = self.resource_dependencies(current.as_ref()).await?;
        self.resources_observed()?
            .entry(target)
            .or_insert_with(|| AgentObservation::new(None))
            .publish(current.clone());
        Ok(json!({"document": current, "referenced_by": dependencies}).to_string())
    }

    async fn checked_resource(&self, target: &ResourceTarget) -> AgentResult<ResourceDocument> {
        let expected = self
            .resources_observed()?
            .get(target)
            .and_then(|value| value.known())
            .cloned();
        let current = self.read_resource(target).await?;
        if expected.as_ref() != Some(&current) {
            let mut observations = self.resources_observed()?;
            let observed = observations
                .entry(target.clone())
                .or_insert_with(|| AgentObservation::new(None));
            observed.invalidate();
            observed.publish(current.clone());
            drop(observations);
            return Err(AgentError::conflict(json!({"code":"outdated","message":"Review the resource and replan in the next response; no mutation was applied.","document":current}).to_string()));
        }
        current.ok_or_else(|| AgentError::not_found("prompt resource does not exist"))
    }

    pub(super) async fn create_resource(&self, arguments: &str) -> AgentResult<String> {
        let request: CreateResource = parse_args(arguments)?;
        let prompt = self.app.prompt();
        let writer = prompt.write().await;
        let saved = self.save_resource(&writer, request.into()).await?;
        drop(writer);
        self.publish_resource(&saved)
    }

    pub(super) async fn edit_resource(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            target: ResourceTarget,
            operations: Vec<ResourceOperation>,
        }
        let args: Args = parse_args(arguments)?;
        let _draft = self.app.generation_draft_write.lock().await;
        let prompt = self.app.prompt();
        let writer = prompt.write().await;
        let before = self.checked_resource(&args.target).await?;
        let after = before.edit(&args.operations)?;
        if after == before {
            return Ok(json!({"changed":false,"document":before}).to_string());
        }
        let saved = self.save_resource(&writer, after.into()).await?;
        drop(writer);
        self.publish_resource(&saved)
    }

    pub(super) async fn copy_resource(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            target: ResourceTarget,
            path: String,
            folder_id: Option<String>,
            display_name: String,
        }
        let args: Args = parse_args(arguments)?;
        let prompt = self.app.prompt();
        let writer = prompt.write().await;
        let mut request = ResourceWrite::from(self.checked_resource(&args.target).await?);
        request.copy_to(args.path, args.folder_id, args.display_name);
        let saved = self.save_resource(&writer, request).await?;
        drop(writer);
        self.publish_resource(&saved)
    }

    pub(super) async fn delete_resource(&self, arguments: &str) -> AgentResult<String> {
        let target: ResourceTarget = parse_args(arguments)?;
        let _draft = self.app.generation_draft_write.lock().await;
        let prompt = self.app.prompt();
        let writer = prompt.write().await;
        let current = self.checked_resource(&target).await?;
        let dependencies = self.resource_dependencies(Some(&current)).await?;
        if !dependencies.is_empty() {
            return Err(AgentError::conflict(
                json!({"code":"resource_in_use","referenced_by":dependencies}).to_string(),
            ));
        }
        match target.kind {
            ResourceKind::Chunk => {
                writer
                    .delete_chunk(DeletePromptChunkRequestDto {
                        chunk_id: target.id.clone(),
                    })
                    .await
                    .map_err(agent_app_error)?;
            }
            ResourceKind::Preset => {
                writer
                    .delete_preset(DeletePromptPresetRequestDto {
                        preset_id: target.id.clone(),
                    })
                    .await
                    .map_err(agent_app_error)?;
            }
        }
        drop(writer);
        let mut observations = self.resources_observed()?;
        let observed = observations
            .entry(target)
            .or_insert_with(|| AgentObservation::new(None));
        observed.invalidate();
        observed.publish(None);
        drop(observations);
        Ok(json!({"deleted":true}).to_string())
    }

    fn publish_resource(&self, saved: &ResourceDocument) -> AgentResult<String> {
        let mut observations = self.resources_observed()?;
        let observation = observations
            .entry(saved.target())
            .or_insert_with(|| AgentObservation::new(None));
        observation.invalidate();
        observation.publish(Some(saved.clone()));
        drop(observations);
        Ok(json!({"changed":true,"document":saved}).to_string())
    }

    async fn resource_dependencies(
        &self,
        resource: Option<&ResourceDocument>,
    ) -> AgentResult<Vec<serde_json::Value>> {
        match resource {
            Some(ResourceDocument::Chunk(chunk)) => {
                let key = atelier_prompt_resources::PromptChunkKey::parse(&chunk.path)
                    .map_err(|error| AgentError::validation(error.to_string()))?;
                Ok(self
                    .app
                    .prompt_chunks
                    .list_references(&key)
                    .await
                    .map_err(|error| AgentError::runtime(error.to_string()))?
                    .into_iter()
                    .map(|value| json!({"id":value.chunk_id.as_str(),"path":value.key.as_str()}))
                    .collect())
            }
            Some(ResourceDocument::Preset(preset)) => {
                let draft = self
                    .app
                    .generation()
                    .get_draft()
                    .await
                    .map_err(agent_app_error)?;
                Ok(draft.into_iter().flat_map(|draft| draft.draft.prompt_states).filter_map(|state| {
                    let main = state.main_preset_id.as_deref() == Some(&preset.preset_id);
                    let characters = state.characters.iter().filter(|value| value.preset_id.as_deref() == Some(&preset.preset_id)).map(|value| value.id.clone()).collect::<Vec<_>>();
                    (main || !characters.is_empty()).then(|| json!({"draft_model":state.model,"main":main,"character_ids":characters}))
                }).collect())
            }
            None => Ok(Vec::new()),
        }
    }

    async fn save_resource(
        &self,
        writer: &crate::usecases::PromptWriteUseCases<'_>,
        request: ResourceWrite,
    ) -> AgentResult<ResourceDocument> {
        let saved = match request {
            ResourceWrite::Chunk(request) => writer
                .upsert_chunk(request)
                .await
                .map(ResourceDocument::Chunk),
            ResourceWrite::Preset(request) => writer
                .upsert_preset(request)
                .await
                .map(ResourceDocument::Preset),
        }
        .map_err(agent_app_error)?;
        self.read_resource(&saved.target())
            .await?
            .ok_or_else(|| AgentError::not_found("saved resource disappeared"))
    }
}
