use serde::{Deserialize, Deserializer};

/// Distinguishes an omitted field from an explicit value, including null for optional values.
#[derive(Default)]
pub enum Patch<T> {
    #[default]
    Unchanged,
    Set(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Set)
    }
}

impl<T: Clone> Patch<T> {
    pub fn apply(&self, target: &mut T) {
        if let Self::Set(value) = self {
            target.clone_from(value);
        }
    }
}
