use async_trait::async_trait;
use nai_atelier_foundation::NovelAiError;

pub type DirectorResult<T> = Result<T, NovelAiError>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DirectorTool {
    #[default]
    Lineart,
    Sketch,
    BgRemoval,
    Emotion,
    Declutter,
    Colorize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDirectorToolRequest {
    pub tool: DirectorTool,
    pub image: String,
    pub prompt: Option<String>,
    pub defry: Option<u8>,
    pub strict_mode: bool,
}

impl Default for RunDirectorToolRequest {
    fn default() -> Self {
        Self {
            tool: DirectorTool::default(),
            image: String::new(),
            prompt: None,
            defry: None,
            strict_mode: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectorToolOutput {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
    pub seed: Option<i64>,
}

#[async_trait]
pub trait NovelAiDirectorClient: Send + Sync {
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-director");
    }

    #[test]
    fn run_director_tool_request_defaults_to_lineart() {
        let request = RunDirectorToolRequest::default();

        assert_eq!(request.tool, DirectorTool::Lineart);
        assert!(request.strict_mode);
    }
}
