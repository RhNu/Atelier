/// Image content is carried separately from persistable tool-result text.
#[derive(Clone, PartialEq, Eq)]
pub struct AgentToolImage {
    pub media_type: AgentImageMediaType,
    pub base64: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentImageMediaType {
    Png,
    Jpeg,
    Webp,
}

impl std::fmt::Debug for AgentToolImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentToolImage")
            .field("media_type", &self.media_type)
            .field("encoded_bytes", &self.base64.len())
            .finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentToolOutput {
    pub text: String,
    pub images: Vec<AgentToolImage>,
}

impl AgentToolOutput {
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            images: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_payload_is_not_in_debug_output() {
        let image = AgentToolImage {
            media_type: AgentImageMediaType::Png,
            base64: "sensitive-image-payload".into(),
        };
        assert!(!format!("{image:?}").contains(&image.base64));
    }
}
