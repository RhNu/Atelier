#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceLockRequest {
    pub holder: String,
}

impl WorkspaceLockRequest {
    #[must_use]
    pub fn new(holder: impl Into<String>) -> Self {
        Self {
            holder: holder.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceLockMetadata {
    pub holder: String,
    pub created_at_ms: u64,
}

impl WorkspaceLockMetadata {
    #[must_use]
    pub fn new(holder: impl Into<String>, created_at_ms: u64) -> Self {
        Self {
            holder: holder.into(),
            created_at_ms,
        }
    }
}
