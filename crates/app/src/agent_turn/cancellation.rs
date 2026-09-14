use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentResult};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::json;

use super::tools::{AgentTools, agent_app_error, parse_args};

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn cancel_generation(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            batch_id: String,
        }
        let args: Args = parse_args(arguments)?;
        if !self
            .owned_batches
            .lock()
            .map_err(|_| AgentError::runtime("generation ownership unavailable"))?
            .contains(&args.batch_id)
        {
            return Err(AgentError::validation(
                "only a batch submitted in this Agent turn can be cancelled",
            ));
        }
        self.coordinator.generations.cancel(&args.batch_id)?;
        self.app
            .generation()
            .cancel_batch(&args.batch_id)
            .await
            .map_err(agent_app_error)?;
        let state = self.batch_status(&args.batch_id).await?;
        Ok(json!({"state":state}).to_string())
    }

    pub(super) async fn cancel_owned_generations(&self) -> AgentResult<()> {
        let batches = self
            .owned_batches
            .lock()
            .map_err(|_| AgentError::runtime("generation ownership unavailable"))?
            .clone();
        for batch in batches {
            self.coordinator.generations.cancel(&batch)?;
            self.app
                .generation()
                .cancel_batch(&batch)
                .await
                .map_err(agent_app_error)?;
        }
        Ok(())
    }
}
