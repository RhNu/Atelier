#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PromptCapability {
    NumericEmphasis,
    NegativeNumericEmphasis,
    Randomizer,
    MultiCharacterPipe,
    PromptMixingPipe,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptSyntaxProfile {
    name: &'static str,
    capabilities: Vec<PromptCapability>,
}

impl PromptSyntaxProfile {
    #[must_use]
    pub const fn new(name: &'static str, capabilities: Vec<PromptCapability>) -> Self {
        Self { name, capabilities }
    }

    #[must_use]
    pub fn novelai_v3() -> Self {
        Self::new(
            "novelai-v3",
            vec![
                PromptCapability::Randomizer,
                PromptCapability::PromptMixingPipe,
            ],
        )
    }

    #[must_use]
    pub fn novelai_v4() -> Self {
        Self::new(
            "novelai-v4",
            vec![
                PromptCapability::NumericEmphasis,
                PromptCapability::Randomizer,
                PromptCapability::MultiCharacterPipe,
            ],
        )
    }

    #[must_use]
    pub fn novelai_v45() -> Self {
        Self::new(
            "novelai-v4.5",
            vec![
                PromptCapability::NumericEmphasis,
                PromptCapability::NegativeNumericEmphasis,
                PromptCapability::Randomizer,
                PromptCapability::MultiCharacterPipe,
            ],
        )
    }

    #[must_use]
    pub fn novelai_v5() -> Self {
        Self::new(
            "novelai-v5",
            vec![
                PromptCapability::NumericEmphasis,
                PromptCapability::NegativeNumericEmphasis,
                PromptCapability::Randomizer,
                PromptCapability::MultiCharacterPipe,
            ],
        )
    }

    #[must_use]
    pub fn supports(&self, capability: PromptCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }
}
