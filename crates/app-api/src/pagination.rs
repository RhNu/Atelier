use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageQueryDto {
    pub offset: usize,
    pub limit: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageInfoDto {
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

impl PageInfoDto {
    #[must_use]
    pub const fn new(offset: usize, limit: usize, total: usize) -> Self {
        Self {
            offset,
            limit,
            total,
        }
    }
}
