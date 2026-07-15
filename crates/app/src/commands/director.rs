use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::director::{DirectorToolResultDto, RunDirectorToolRequestDto};
use atelier_secrets::SecretStore;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
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
        Self::command_result(self.current_session()?.director().run_tool(request).await)
    }
}
