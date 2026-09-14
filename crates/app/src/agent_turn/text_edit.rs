use atelier_agent::{AgentError, AgentResult};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextEdit {
    Set {
        text: String,
    },
    Replace {
        old_text: String,
        new_text: String,
        #[serde(default)]
        all: bool,
    },
    Append {
        text: String,
    },
}

impl TextEdit {
    pub fn apply(&self, text: &str) -> AgentResult<String> {
        match self {
            Self::Set { text } => Ok(text.clone()),
            Self::Replace {
                old_text,
                new_text,
                all,
            } => atelier_prompt::replace_prompt_text(text, old_text, new_text, *all)
                .map_err(|error| AgentError::validation(error.to_string())),
            Self::Append { text: suffix } => Ok(format!("{text}{suffix}")),
        }
    }
}
