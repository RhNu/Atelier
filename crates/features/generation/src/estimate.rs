const COST_COEFF_A: f64 = 2.951_823_174_884_865e-6;
const COST_COEFF_B: f64 = 5.753_298_233_447_344e-7;
const MIN_RESOLUTION: u64 = 65_536;
const BASE_RESOLUTION: u64 = 832 * 1216;
const OPUS_DISCOUNT_MAX_RESOLUTION: u64 = 1024 * 1024;
const DIRECTOR_REFERENCE_EXTRA_COST: u64 = 5;
const VIBE_ENCODE_EXTRA_COST: u64 = 2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AnlasEstimateInput {
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub n_samples: u32,
    pub request_count: u32,
    pub has_img2img: bool,
    pub img2img_strength: f32,
    pub has_director_reference: bool,
    pub pending_encode_count: u32,
    pub is_opus: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnlasEstimate {
    pub per_sample_cost: u64,
    pub per_request_cost: u64,
    pub total_cost: u64,
    pub adjusted_resolution: u64,
    pub opus_discount_applied: bool,
    pub pending_encode_cost: u64,
}

/// Estimates `NovelAI` Anlas cost for a normalized generation request.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn estimate_anlas_cost(input: AnlasEstimateInput) -> AnlasEstimate {
    let width = positive_u32(input.width);
    let height = positive_u32(input.height);
    let steps = positive_u32(input.steps);
    let n_samples = positive_u32(input.n_samples);
    let request_count = positive_u32(input.request_count);
    let img2img_strength = clamp_f32(input.img2img_strength, 0.01, 0.99, 0.7);

    let resolution = u64::from(width)
        .saturating_mul(u64::from(height))
        .max(MIN_RESOLUTION);
    let adjusted_resolution =
        if resolution > BASE_RESOLUTION && resolution <= OPUS_DISCOUNT_MAX_RESOLUTION {
            BASE_RESOLUTION
        } else {
            resolution
        };
    let strength_multiplier = if input.has_img2img {
        f64::from(img2img_strength)
    } else {
        1.0
    };

    let raw_sample_cost = (COST_COEFF_A.mul_add(
        adjusted_resolution as f64,
        COST_COEFF_B * adjusted_resolution as f64 * f64::from(steps),
    ))
    .ceil() as u64;
    let per_sample_cost = ((raw_sample_cost as f64) * strength_multiplier)
        .ceil()
        .max(2.0) as u64;

    let opus_discount_applied =
        input.is_opus && steps <= 28 && adjusted_resolution <= OPUS_DISCOUNT_MAX_RESOLUTION;
    let payable_samples = if opus_discount_applied { 0 } else { n_samples };
    let mut per_request_cost = per_sample_cost.saturating_mul(u64::from(payable_samples));
    if input.has_director_reference {
        per_request_cost = per_request_cost.saturating_add(DIRECTOR_REFERENCE_EXTRA_COST);
    }

    let pending_encode_cost =
        u64::from(input.pending_encode_count).saturating_mul(VIBE_ENCODE_EXTRA_COST);
    let total_cost = per_request_cost
        .saturating_mul(u64::from(request_count))
        .saturating_add(pending_encode_cost);

    AnlasEstimate {
        per_sample_cost,
        per_request_cost,
        total_cost,
        adjusted_resolution,
        opus_discount_applied,
        pending_encode_cost,
    }
}

const fn positive_u32(value: u32) -> u32 {
    if value == 0 { 1 } else { value }
}

const fn clamp_f32(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}
