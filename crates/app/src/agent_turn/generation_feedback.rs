use std::time::Duration;

use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentResult, AgentToolSpec};
use atelier_app_api::history::{
    GenerationBatchHistoryStatusDto, GenerationHistoryBatchDetailDto,
    GenerationHistoryBatchRequestDto,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::json;

use super::{
    schemas::{object, spec},
    tools::{AgentTools, agent_app_error, parse_args},
};

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn batch_status(
        &self,
        batch_id: &str,
    ) -> AgentResult<GenerationHistoryBatchDetailDto> {
        if !self
            .generation_batches
            .lock()
            .map_err(|_| AgentError::runtime("generation ownership unavailable"))?
            .contains(batch_id)
        {
            return Err(AgentError::validation(
                "batch is not available in this Agent turn",
            ));
        }
        self.app
            .history()
            .get_generation_batch(GenerationHistoryBatchRequestDto {
                batch_id: batch_id.to_owned(),
            })
            .await
            .map_err(agent_app_error)
    }

    pub(super) async fn generation_status(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            batch_id: String,
        }
        let args: Args = parse_args(arguments)?;
        let state = self.batch_status(&args.batch_id).await?;
        Ok(json!({"terminal":terminal(state.batch.status),"state":state}).to_string())
    }

    pub(super) async fn wait_generation(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            batch_id: String,
            timeout_seconds: Option<u64>,
        }
        let args: Args = parse_args(arguments)?;
        let seconds = args.timeout_seconds.unwrap_or(30);
        if !(1..=60).contains(&seconds) {
            return Err(AgentError::validation(
                "timeout_seconds must be between 1 and 60",
            ));
        }
        let mut changes = self.app.events.subscribe_changes();
        let deadline = tokio::time::sleep(Duration::from_secs(seconds));
        tokio::pin!(deadline);
        loop {
            changes.borrow_and_update();
            let state = self.batch_status(&args.batch_id).await?;
            if terminal(state.batch.status) {
                return Ok(json!({"terminal":true,"timed_out":false,"state":state}).to_string());
            }
            tokio::select! {
                () = self.cancellation.cancelled() => return Err(AgentError::cancelled()),
                () = &mut deadline => {
                    let state = self.batch_status(&args.batch_id).await?;
                    return Ok(json!({"terminal":terminal(state.batch.status),"timed_out":!terminal(state.batch.status),"state":state}).to_string());
                }
                notification = changes.changed() => {
                    notification.map_err(|_| AgentError::runtime("generation event channel closed"))?;
                }
            }
        }
    }
}

const fn terminal(status: GenerationBatchHistoryStatusDto) -> bool {
    matches!(
        status,
        GenerationBatchHistoryStatusDto::Succeeded
            | GenerationBatchHistoryStatusDto::PartiallySucceeded
            | GenerationBatchHistoryStatusDto::Failed
            | GenerationBatchHistoryStatusDto::Stopped
    )
}

pub fn specs() -> Vec<AgentToolSpec> {
    vec![
        spec(
            "cancel_generation",
            "Cancel only a batch submitted in this Agent turn, including pending requests. Existing completed outputs are preserved. This does not stop unrelated user batches.",
            object(json!({"batch_id":{"type":"string"}}), &["batch_id"]),
        ),
        spec(
            "get_generation_status",
            "Read an output-area or Agent-submitted batch's request statuses, failures and available outputs. Output references are metadata; this tool does not inspect pixels.",
            object(json!({"batch_id":{"type":"string"}}), &["batch_id"]),
        ),
        spec(
            "wait_for_generation",
            "Wait for an output-area or Agent-submitted batch to finish using generation events. Timeout is not completion; inspect terminal and timed_out. Paused or queued batches may require another wait or user action.",
            object(
                json!({"batch_id":{"type":"string"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":60}}),
                &["batch_id"],
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{GenerationBatchHistoryStatusDto as Status, terminal};

    #[test]
    fn pending_states_are_not_terminal() {
        for status in [
            Status::Queued,
            Status::Preparing,
            Status::Running,
            Status::Waiting,
            Status::Paused,
        ] {
            assert!(!terminal(status));
        }
    }

    #[test]
    fn all_finished_outcomes_are_terminal() {
        for status in [
            Status::Succeeded,
            Status::PartiallySucceeded,
            Status::Failed,
            Status::Stopped,
        ] {
            assert!(terminal(status));
        }
    }
}
