use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{
    AgentError, AgentImageMediaType, AgentResult, AgentToolImage, AgentToolOutput, AgentToolSpec,
};
use atelier_app_api::{history::RunHistoryOutputStateDto, resource::GetResourceImageRequestDto};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::json;

use super::{
    schemas::object,
    tools::{AgentTools, agent_app_error, parse_args},
};

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn read_generation_output(
        &self,
        arguments: &str,
    ) -> AgentResult<AgentToolOutput> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            batch_id: String,
            job_id: String,
            sample_index: u32,
        }
        let args: Args = parse_args(arguments)?;
        self.require_output_vision().await?;
        let batch = self.batch_status(&args.batch_id).await?;
        let output = batch
            .requests
            .iter()
            .find(|request| request.job_id == args.job_id)
            .and_then(|request| {
                request
                    .outputs
                    .iter()
                    .find(|output| output.sample_index == Some(args.sample_index))
            })
            .filter(|output| output.state == RunHistoryOutputStateDto::Available)
            .ok_or_else(|| {
                AgentError::not_found(
                    "output is not yet available or was deleted; read generation status",
                )
            })?;
        let resource = output
            .resource
            .clone()
            .ok_or_else(|| AgentError::not_found("output resource is unavailable"))?;
        let image = self
            .app
            .resources()
            .get_image(GetResourceImageRequestDto { resource })
            .await
            .map_err(agent_app_error)?;
        let media_type = match image.mime_type.as_deref() {
            Some("image/png") => AgentImageMediaType::Png,
            Some("image/jpeg") => AgentImageMediaType::Jpeg,
            Some("image/webp") => AgentImageMediaType::Webp,
            _ => {
                return Err(AgentError::validation(
                    "output image format is not supported by the Agent",
                ));
            }
        };
        self.require_output_vision().await?;
        Ok(AgentToolOutput {
            text: json!({"batch_id":args.batch_id,"job_id":args.job_id,"sample_index":args.sample_index,"artifact_id":output.artifact_id,"image_attached":true}).to_string(),
            images: vec![AgentToolImage { media_type, base64: image.image_base64 }],
        })
    }

    async fn require_output_vision(&self) -> AgentResult<()> {
        if self.cancellation.is_cancelled() {
            return Err(AgentError::cancelled());
        }
        if !self.vision_enabled || !self.app.agent.get_settings().await?.output_vision_enabled {
            return Err(AgentError::validation(
                "output vision must be enabled by the user with an image-capable model before starting the turn",
            ));
        }
        Ok(())
    }
}

pub fn spec() -> AgentToolSpec {
    super::schemas::spec(
        "read_generation_output",
        "Read the actual pixels of one completed output from the user's output area or a batch submitted in this turn. Requires user-enabled output vision and an image-capable model. Read status to find job_id and sample_index. This cannot read arbitrary resources or modify images.",
        object(
            json!({"batch_id":{"type":"string"},"job_id":{"type":"string"},"sample_index":{"type":"integer","minimum":0}}),
            &["batch_id", "job_id", "sample_index"],
        ),
    )
}

pub(super) const fn vision_allowed(user_enabled: bool, model_supports_images: bool) -> bool {
    user_enabled && model_supports_images
}

#[cfg(test)]
mod tests {
    use super::vision_allowed;

    #[test]
    fn pixels_require_both_user_opt_in_and_model_capability() {
        assert!(!vision_allowed(false, false));
        assert!(!vision_allowed(false, true));
        assert!(!vision_allowed(true, false));
        assert!(vision_allowed(true, true));
    }
}
