import pytest

from atelier_lexicon.enrich import _validate
from atelier_lexicon.relations import normalized_pmi


def test_llm_output_requires_structured_three_state_rating() -> None:
    _validate(
        {
            "chinese_translation": "红发",
            "chinese_summary": "红色头发。",
            "expansion_terms": ["red hair"],
            "rating": "unknown",
        }
    )
    with pytest.raises(ValueError, match="rating"):
        _validate(
            {
                "chinese_translation": "红发",
                "chinese_summary": "红色头发。",
                "expansion_terms": [],
                "rating": "nsfw",
            }
        )


def test_npmi_is_clipped_and_empty_observations_are_zero() -> None:
    assert normalized_pmi(0, 10, 10, 100) == 0
    assert -1 <= normalized_pmi(10, 10, 10, 100) <= 1
