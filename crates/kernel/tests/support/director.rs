use async_trait::async_trait;
use atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};

use super::MemoryKernelPorts;

#[async_trait]
impl NovelAiDirectorClient for MemoryKernelPorts {
    async fn run_director_tool(
        &self,
        _request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("run_director_tool".to_owned());
        Ok(state
            .director_output
            .clone()
            .unwrap_or_else(|| DirectorToolOutput {
                bytes: vec![4, 5, 6],
                mime_type: Some("image/png".to_owned()),
                seed: None,
            }))
    }
}
