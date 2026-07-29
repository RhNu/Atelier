import pytest

from atelier_lexicon.aliases import resolve_aliases


def test_alias_chains_resolve_to_canonical_target() -> None:
    result = resolve_aliases(
        [("miku", "hatsune_miku"), ("初音", "miku")],
        {"hatsune_miku"},
    )
    assert result == {"miku": "hatsune_miku", "初音": "hatsune_miku"}


def test_alias_cycles_are_rejected() -> None:
    with pytest.raises(ValueError, match="cycle"):
        resolve_aliases([("a", "b"), ("b", "a")], {"a", "b"})
