use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationErrorKind {
    EmptyField,
    NumericOutOfRange,
    NonFiniteNumber,
    InvalidImageDimension,
    UnsupportedModelFeature,
    UnsupportedFieldCombination,
}

impl std::fmt::Display for GenerationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EmptyField => "empty_field",
            Self::NumericOutOfRange => "numeric_out_of_range",
            Self::NonFiniteNumber => "non_finite_number",
            Self::InvalidImageDimension => "invalid_image_dimension",
            Self::UnsupportedModelFeature => "unsupported_model_feature",
            Self::UnsupportedFieldCombination => "unsupported_field_combination",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct GenerationError {
    pub kind: GenerationErrorKind,
    pub field: Option<String>,
    pub message: String,
}

impl GenerationError {
    #[must_use]
    pub fn empty_field(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::EmptyField,
            Some(field.clone()),
            format!("{field} cannot be empty"),
        )
    }

    #[must_use]
    pub fn numeric_out_of_range(
        field: impl Into<String>,
        min: impl std::fmt::Display,
        max: impl std::fmt::Display,
    ) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::NumericOutOfRange,
            Some(field.clone()),
            format!("{field} must be between {min} and {max}"),
        )
    }

    #[must_use]
    pub fn non_finite_number(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::NonFiniteNumber,
            Some(field.clone()),
            format!("{field} must be finite"),
        )
    }

    #[must_use]
    pub fn invalid_image_dimension(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::InvalidImageDimension,
            Some(field.clone()),
            format!("{field} must be in [64,1600] and a multiple of 64"),
        )
    }

    #[must_use]
    pub fn unsupported_model_feature(
        field: impl Into<String>,
        required_model: impl std::fmt::Display,
    ) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::UnsupportedModelFeature,
            Some(field.clone()),
            format!("{field} requires {required_model}"),
        )
    }

    #[must_use]
    pub fn unsupported_field_combination(
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            GenerationErrorKind::UnsupportedFieldCombination,
            Some(field.into()),
            message,
        )
    }

    #[must_use]
    pub fn new(
        kind: GenerationErrorKind,
        field: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            field,
            message: message.into(),
        }
    }
}
