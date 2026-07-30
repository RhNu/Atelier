from atelier_nsfw_benchmark.production_compare import POLICY, policy_decision


def test_production_comparison_reads_the_versioned_rust_policy_asset() -> None:
    assert POLICY["id"] == "anime-rating-cascade"
    assert POLICY["version"] == "1"
    assert POLICY["review_model_revision"] == "627aef95638667ddcaa3ac8ae625e88ea5b02f51"


def test_policy_decision_skips_review_for_direct_primary_result() -> None:
    decision, reviewed = policy_decision(
        {
            "dbrating_general": "0.99",
            "dbrating_sensitive": "0.0",
            "dbrating_questionable": "0.005",
            "dbrating_explicit": "0.005",
            "wd_general": "0.0",
            "wd_sensitive": "0.0",
            "wd_questionable": "0.0",
            "wd_explicit": "0.0",
        }
    )

    assert decision is False
    assert reviewed is False
