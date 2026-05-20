use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::director::{DirectorToolResultDto, RunDirectorToolRequestDto};
use nai_atelier_secrets::SecretStore;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    /// Runs one `NovelAI` Director tool and indexes the resulting image.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no API key is active, or Director execution fails.
    pub async fn run_director_tool(
        &self,
        request: RunDirectorToolRequestDto,
    ) -> CommandResult<DirectorToolResultDto> {
        Self::command_result(self.current_app()?.director().run_tool(request).await)
    }
}
