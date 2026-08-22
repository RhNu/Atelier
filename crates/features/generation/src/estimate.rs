//! Anlas estimate result model.
//!
//! The pricing formula itself lives in `novelai-bridge` and is documented in that crate's
//! `docs/anlas-pricing.md`. `adapters/novelai` maps a normalized request onto the bridge
//! calculator and returns the types below, so this crate never carries a second copy of the
//! formula.

/// Whether the `NovelAI` web client would accept the per-image price of a request.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AnlasEstimateStatus {
    /// The per-image price is within the accepted limit.
    #[default]
    Available,
    /// The per-image price exceeds the limit the web client accepts.
    TooExpensive,
}

/// Estimated Anlas cost of a planned generation run.
///
/// A run is `request_count` identical requests. Per-request components are already multiplied by
/// `request_count`; `pending_encode_cost` is charged once for the whole run because a Vibe
/// encoding is cached after its first `/ai/encode-vibe` call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnlasEstimate {
    /// Whether the per-image price is acceptable.
    pub status: AnlasEstimateStatus,
    /// Price of a single image before the Opus first-image allowance.
    pub per_image_cost: u64,
    /// Cost of one request: generation plus character-reference and Vibe overage surcharges.
    pub per_request_cost: u64,
    /// Number of requests in the run.
    pub request_count: u32,
    /// Generation cost across the run.
    pub generation_cost: u64,
    /// Character-reference surcharge across the run.
    pub character_reference_cost: u64,
    /// Surcharge for active Vibe references beyond the included count, across the run.
    pub vibe_reference_overage_cost: u64,
    /// One-off cost of the Vibe encodings this run still has to perform.
    pub pending_encode_cost: u64,
    /// Sum of every component.
    pub total_cost: u64,
    /// Images requested per request before any cap.
    pub requested_samples: u32,
    /// Maximum number of images priced at the requested output resolution.
    pub sample_limit: u32,
    /// Images retained per request after the API and resolution caps.
    pub priced_samples: u32,
    /// Images charged per request after the Opus first-image allowance.
    pub billable_samples: u32,
    /// Whether the Opus first-image allowance applies to each request.
    pub free_first_image_applied: bool,
}

impl AnlasEstimate {
    /// Returns `true` when the run is priceable and costs nothing.
    #[must_use]
    pub const fn is_free(&self) -> bool {
        matches!(self.status, AnlasEstimateStatus::Available)
            && self.priced_samples > 0
            && self.total_cost == 0
    }

    /// Returns `true` when the per-image price is above what the web client accepts.
    #[must_use]
    pub const fn is_too_expensive(&self) -> bool {
        matches!(self.status, AnlasEstimateStatus::TooExpensive)
    }
}
