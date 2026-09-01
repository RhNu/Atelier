//! Anlas cost estimation, delegated to `novelai-bridge`.
//!
//! The pricing formula lives in the bridge (`docs/anlas-pricing.md` in that crate). This module
//! assembles the same request the client would send and hands it to
//! [`bridge::GenerateImageRequest::anlas_estimate_input`], so capability gating — inpainting
//! suppressing character references, models without Vibe support, SMEA availability — is derived
//! from the real request rather than re-implemented here.

use super::{
    AnlasEstimate, AnlasEstimateStatus, GenerateImageRequest, GenerationPlanContext,
    GenerationResult, bridge, map_bridge_error, map_generation_error,
    mapping::to_bridge_generate_request,
};

/// Estimates the Anlas cost of a planned generation run.
///
/// `context.request_count` identical requests are priced; per-request components are multiplied
/// out while the Vibe encoding charge is applied once, because an encoding is cached after its
/// first `/ai/encode-vibe` call.
///
/// # Errors
/// Returns a generation client error when an image or Vibe payload is invalid, or when the bridge
/// cannot normalize the request for pricing.
pub fn estimate_anlas_cost(
    request: &GenerateImageRequest,
    context: GenerationPlanContext,
) -> GenerationResult<AnlasEstimate> {
    let bridge_request = to_bridge_generate_request(request.clone())
        .map_err(|error| map_generation_error(map_bridge_error(error)))?;
    let mut input = bridge_request
        .anlas_estimate_input(to_bridge_pricing_context(context))
        .map_err(|error| map_generation_error(map_bridge_error(error)))?;
    input.pending_vibe_encodes = context.pending_vibe_encode_count;
    Ok(from_bridge_anlas_estimate(
        bridge::estimate_anlas_cost(input),
        context.request_count,
    ))
}

const fn to_bridge_pricing_context(context: GenerationPlanContext) -> bridge::AnlasPricingContext {
    bridge::AnlasPricingContext {
        tier: context.tier,
        subscription_active: context.subscription_active,
        v5_usage_is_negative: context.v5_usage_is_negative,
        // Atelier prices each request on its own, so no earlier request in the run has consumed
        // the Opus first-image allowance.
        free_first_image_used: false,
    }
}

fn from_bridge_anlas_estimate(
    estimate: bridge::AnlasEstimate,
    request_count: u32,
) -> AnlasEstimate {
    let request_count = request_count.max(1);
    let runs = u64::from(request_count);
    let generation = u64::from(estimate.generation);
    let character_reference = u64::from(estimate.character_reference);
    let vibe_reference_overage = u64::from(estimate.vibe_reference_overage);
    let per_request_cost = generation
        .saturating_add(character_reference)
        .saturating_add(vibe_reference_overage);
    let pending_encode_cost = u64::from(estimate.vibe_encoding);

    AnlasEstimate {
        status: from_bridge_anlas_estimate_status(estimate.status),
        per_image_cost: u64::from(estimate.per_image),
        per_request_cost,
        request_count,
        generation_cost: generation.saturating_mul(runs),
        character_reference_cost: character_reference.saturating_mul(runs),
        vibe_reference_overage_cost: vibe_reference_overage.saturating_mul(runs),
        pending_encode_cost,
        total_cost: per_request_cost
            .saturating_mul(runs)
            .saturating_add(pending_encode_cost),
        requested_samples: estimate.requested_samples,
        sample_limit: estimate.sample_limit,
        priced_samples: estimate.priced_samples,
        billable_samples: estimate.billable_samples,
        free_first_image_applied: estimate.free_first_image_applied,
    }
}

const fn from_bridge_anlas_estimate_status(
    status: bridge::AnlasEstimateStatus,
) -> AnlasEstimateStatus {
    match status {
        bridge::AnlasEstimateStatus::Available => AnlasEstimateStatus::Available,
        bridge::AnlasEstimateStatus::TooExpensive => AnlasEstimateStatus::TooExpensive,
    }
}
