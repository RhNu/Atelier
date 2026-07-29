import json
from pathlib import Path

import pyarrow.parquet as parquet

from atelier_lexicon.io import sha256_file
from atelier_lexicon.normalize import normalize


def test_selection_uses_real_ids_thresholds_lists_and_artist_partition(
    tmp_path: Path,
) -> None:
    input_dir = tmp_path / "input"
    input_dir.mkdir()
    (input_dir / "tags_enhanced.csv").write_text(
        "name,category,post_count,cn_name,wiki,nsfw\n"
        "popular,0,100,热门,,false\n"
        "cold,0,99,冷门,,false\n"
        "series,3,100,作品,,false\n"
        "allowed_character,4,1,角色,,false\n"
        "some_artist,1,5,画师,,false\n"
        "deleted,0,1000,已删除,,false\n",
        encoding="utf-8",
    )
    snapshot = [
        _tag(11, "popular", 0, 100),
        _tag(12, "cold", 0, 99),
        _tag(13, "series", 3, 100),
        _tag(14, "allowed_character", 4, 1),
        _tag(15, "some_artist", 1, 5),
        _tag(16, "deleted", 0, 1000, deleted=True),
    ]
    (input_dir / "tags.jsonl").write_text(
        "".join(json.dumps(row, sort_keys=True) + "\n" for row in snapshot),
        encoding="utf-8",
    )
    (input_dir / "danbooru-tags-source.json").write_text(
        json.dumps({"retrieved_at": "2026-07-29T00:00:00Z"}),
        encoding="utf-8",
    )
    allow = tmp_path / "allow.json"
    deny = tmp_path / "deny.json"
    allow.write_text('["allowed_character"]', encoding="utf-8")
    deny.write_text("[]", encoding="utf-8")

    first = tmp_path / "first"
    second = tmp_path / "second"
    normalize(input_dir, first, allow, deny)
    normalize(input_dir, second, allow, deny)

    entities = [
        json.loads(line)
        for line in (first / "entities.jsonl").read_text(encoding="utf-8").splitlines()
    ]
    assert [(row["id"], row["canonical_name"], row["kind"]) for row in entities] == [
        (11, "popular", "tag"),
        (13, "series", "tag"),
        (14, "allowed_character", "tag"),
        (15, "some_artist", "artist"),
    ]
    cold = parquet.read_table(first / "cold" / "entities.parquet").to_pylist()
    assert {row["canonical_name"] for row in cold} == {"cold", "deleted"}
    for relative in [
        "entities.jsonl",
        "groups.json",
        "relations.jsonl",
        "provenance.json",
        "cold/entities.parquet",
    ]:
        assert sha256_file(first / relative) == sha256_file(second / relative)


def _tag(
    entity_id: int,
    name: str,
    category: int,
    post_count: int,
    deleted: bool = False,
) -> dict[str, object]:
    return {
        "id": entity_id,
        "name": name,
        "category": category,
        "post_count": post_count,
        "is_deleted": deleted,
    }
