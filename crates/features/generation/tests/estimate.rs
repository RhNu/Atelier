use nai_atelier_generation::{AnlasEstimateInput, estimate_anlas_cost};

#[test]
fn applies_base_resolution_normalization_for_square_canvas() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 1024,
        height: 1024,
        steps: 28,
        n_samples: 1,
        request_count: 1,
        has_img2img: false,
        img2img_strength: 0.7,
        has_director_reference: false,
        pending_encode_count: 0,
        is_opus: false,
    });

    assert_eq!(result.adjusted_resolution, 832 * 1216);
    assert_eq!(result.per_request_cost, 20);
    assert_eq!(result.total_cost, 20);
}

#[test]
fn keeps_large_portrait_resolution_without_down_adjustment() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 1024,
        height: 1536,
        steps: 28,
        n_samples: 1,
        request_count: 1,
        has_img2img: false,
        img2img_strength: 0.7,
        has_director_reference: false,
        pending_encode_count: 0,
        is_opus: false,
    });

    assert_eq!(result.adjusted_resolution, 1024 * 1536);
    assert_eq!(result.per_request_cost, 30);
    assert_eq!(result.total_cost, 30);
}

#[test]
fn applies_img2img_strength_multiplier() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 1024,
        height: 1024,
        steps: 28,
        n_samples: 1,
        request_count: 1,
        has_img2img: true,
        img2img_strength: 0.7,
        has_director_reference: false,
        pending_encode_count: 0,
        is_opus: false,
    });

    assert_eq!(result.per_sample_cost, 14);
    assert_eq!(result.per_request_cost, 14);
    assert_eq!(result.total_cost, 14);
}

#[test]
fn applies_opus_free_image_discount_when_conditions_match() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 1024,
        height: 1024,
        steps: 28,
        n_samples: 2,
        request_count: 1,
        has_img2img: false,
        img2img_strength: 0.7,
        has_director_reference: false,
        pending_encode_count: 0,
        is_opus: true,
    });

    assert!(result.opus_discount_applied);
    assert_eq!(result.per_request_cost, 20);
    assert_eq!(result.total_cost, 20);
}

#[test]
fn multiplies_per_request_cost_by_request_count_only() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 832,
        height: 1216,
        steps: 23,
        n_samples: 1,
        request_count: 3,
        has_img2img: false,
        img2img_strength: 0.7,
        has_director_reference: false,
        pending_encode_count: 0,
        is_opus: false,
    });

    assert_eq!(result.per_request_cost, 17);
    assert_eq!(result.total_cost, 51);
}

#[test]
fn adds_director_reference_and_pending_vibe_encode_costs() {
    let result = estimate_anlas_cost(AnlasEstimateInput {
        width: 1024,
        height: 1024,
        steps: 28,
        n_samples: 2,
        request_count: 3,
        has_img2img: false,
        img2img_strength: 0.7,
        has_director_reference: true,
        pending_encode_count: 2,
        is_opus: false,
    });

    assert_eq!(result.per_request_cost, 45);
    assert_eq!(result.pending_encode_cost, 4);
    assert_eq!(result.total_cost, 139);
}
