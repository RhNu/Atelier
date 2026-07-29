from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import pyarrow as arrow
import pyarrow.parquet as parquet

from .aliases import resolve_aliases
from .io import read_string_set, sha256_file, write_json, write_jsonl
from .models import Entity, LocalizedText, Relation
from .relations import normalized_pmi

CATEGORY = {0: "general", 1: "artist", 3: "copyright", 4: "character"}
MAIN_CATEGORIES = {"general", "copyright", "character"}
MIN_POST_COUNT = 100
MAX_RELATIONS_PER_ENTITY = 16
DSO_DATA_FILES = (
    "cooccurrence_clean.parquet",
    "tag_aliases.parquet",
    "tag_artist_cooc.parquet",
    "tag_groups.json",
    "tags_enhanced.csv",
)
DIRECT_DANBOORU_FILES = (
    "danbooru-tags-source.json",
    "tags-missing.json",
    "tags.jsonl",
)


def normalize(
    input_dir: Path,
    output_dir: Path,
    allow_file: Path,
    deny_file: Path,
    dso_input_dir: Path | None = None,
) -> None:
    dso_input_dir = dso_input_dir or input_dir
    allow = read_string_set(allow_file)
    deny = read_string_set(deny_file)
    raw = _read_tags(dso_input_dir / "tags_enhanced.csv")
    for name in deny:
        raw.pop(name, None)
    tag_snapshot = _read_tag_snapshot(input_dir / "tags.jsonl")
    _merge_authoritative_tag_fields(raw, tag_snapshot)
    canonical_names = set(raw)
    aliases = _read_aliases(dso_input_dir / "tag_aliases.parquet", canonical_names)
    aliases_by_target: dict[str, list[str]] = defaultdict(list)
    for alias, target in aliases.items():
        aliases_by_target[target].append(alias)
    groups_by_tag, group_records = _read_groups(dso_input_dir / "tag_groups.json")

    main: list[Entity] = []
    cold: list[Entity] = []
    for name in sorted(raw, key=lambda item: int(raw[item]["id"])):
        row = raw[name]
        category = CATEGORY.get(int(row["category"]))
        if category is None:
            continue
        entity = _entity(
            int(row["id"]),
            name,
            row,
            category,
            aliases_by_target[name],
            groups_by_tag.get(name, []),
        )
        selected = (
            category == "artist"
            or (
                category in MAIN_CATEGORIES
                and (entity.post_count >= MIN_POST_COUNT or name in allow)
            )
        ) and row.get("is_deleted") != "true" and name not in deny
        (main if selected else cold).append(entity)

    selected_ids = {entity.canonical_name: entity.id for entity in main}
    selected_counts = {entity.canonical_name: entity.post_count for entity in main}
    total_posts = max(selected_counts.values(), default=1)
    relations = _read_relations(
        dso_input_dir / "cooccurrence_clean.parquet",
        selected_ids,
        selected_counts,
        total_posts,
        "npmi",
    )
    artist_relations = _read_relations(
        dso_input_dir / "tag_artist_cooc.parquet",
        selected_ids,
        selected_counts,
        total_posts,
        "artist_npmi",
    )
    relations.extend(artist_relations)

    output_dir.mkdir(parents=True, exist_ok=True)
    write_jsonl(output_dir / "entities.jsonl", (entity.to_json() for entity in main))
    write_jsonl(output_dir / "relations.jsonl", (relation.__dict__ for relation in relations))
    write_json(
        output_dir / "groups.json",
        [
            {
                "id": group_id,
                "name": record["name"],
                "members": [
                    selected_ids[name]
                    for name in record["members"]
                    if name in selected_ids
                ],
            }
            for group_id, record in sorted(group_records.items())
        ],
    )
    cold_path = output_dir / "cold" / "entities.parquet"
    cold_path.parent.mkdir(parents=True, exist_ok=True)
    parquet.write_table(
        arrow.Table.from_pylist([entity.to_json() for entity in cold]),
        cold_path,
        compression="zstd",
        use_dictionary=True,
    )
    source_metadata = _source_metadata(input_dir / "danbooru-tags-source.json")
    dso_metadata = _source_metadata(dso_input_dir / "SOURCE.json")
    source_paths = {
        path.resolve(): path
        for path in (
            *[
                input_dir / name
                for name in DIRECT_DANBOORU_FILES
                if (input_dir / name).is_file()
            ],
            *[
                dso_input_dir / name
                for name in DSO_DATA_FILES
                if (dso_input_dir / name).is_file()
            ],
        )
    }
    write_json(
        output_dir / "provenance.json",
        {
            "generated_at": source_metadata.get("retrieved_at", "1970-01-01T00:00:00Z"),
            "pipeline_version": "1",
            "selection": {
                "categories": sorted(MAIN_CATEGORIES),
                "minimum_post_count": MIN_POST_COUNT,
                "artist_partition": True,
                "allow_count": len(allow),
                "deny_count": len(deny),
            },
            "counts": {
                "main": len(main),
                "cold": len(cold),
                "relations": len(relations),
            },
            "sources": [
                _provenance_source(
                    path,
                    (
                        source_metadata.get("retrieved_at", "local-pinned-input")
                        if path.name in DIRECT_DANBOORU_FILES
                        else dso_metadata.get("commit", "local-pinned-input")
                    ),
                )
                for path in sorted(source_paths.values())
            ],
        },
    )


def _provenance_source(path: Path, snapshot: str) -> dict[str, str]:
    if path.name in DIRECT_DANBOORU_FILES:
        return {
            "id": path.name,
            "url": "https://danbooru.donmai.us/terms_of_service",
            "snapshot": snapshot,
            "sha256": sha256_file(path),
            "license": "Danbooru Terms of Service",
        }
    return {
        "id": path.name,
        "url": "https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline",
        "snapshot": snapshot,
        "sha256": sha256_file(path),
        "license": "GPL-3.0",
    }


def _read_tags(path: Path) -> dict[str, dict[str, str]]:
    encoding = "utf-8-sig"
    try:
        path.read_text(encoding=encoding)
    except UnicodeDecodeError:
        encoding = "gb18030"
    with path.open("r", encoding=encoding, newline="") as stream:
        return {
            row["name"]: row
            for row in csv.DictReader(stream)
            if row.get("name") and row.get("category") in {"0", "1", "3", "4"}
        }


def _read_tag_snapshot(path: Path) -> dict[str, dict[str, Any]]:
    if not path.is_file():
        raise ValueError(
            f"{path} is required; run `atelier-lexicon fetch-tags` so entity IDs "
            "remain true Danbooru tag IDs"
        )
    result: dict[str, dict[str, Any]] = {}
    with path.open("r", encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            value = json.loads(line)
            name = str(value["name"])
            if name in result:
                raise ValueError(f"{path}:{line_number} duplicates tag {name}")
            result[name] = value
    return result


def _merge_authoritative_tag_fields(
    enhanced: dict[str, dict[str, str]], snapshot: dict[str, dict[str, Any]]
) -> None:
    missing = sorted(name for name in enhanced if name not in snapshot)
    if missing:
        preview = ", ".join(missing[:5])
        raise ValueError(
            f"Danbooru snapshot is missing {len(missing)} enhanced tags, including {preview}"
        )
    for name, row in enhanced.items():
        source = snapshot[name]
        row["id"] = str(int(source["id"]))
        row["category"] = str(int(source["category"]))
        row["post_count"] = str(int(source["post_count"]))
        row["is_deleted"] = "true" if source.get("is_deleted") else "false"


def _source_metadata(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def _entity(
    entity_id: int,
    name: str,
    row: dict[str, str],
    category: str,
    aliases: list[str],
    groups: list[str],
) -> Entity:
    cn_terms = list(
        dict.fromkeys(
            value.strip()
            for value in row.get("cn_name", "").split(",")
            if value.strip()
        )
    )
    primary = cn_terms[0] if cn_terms else name
    translations = [LocalizedText("zh-CN", term) for term in cn_terms]
    wiki = row.get("wiki", "").strip()
    return Entity(
        id=entity_id,
        canonical_name=name,
        primary_translation=primary,
        kind="artist" if category == "artist" else "tag",
        category=category,
        post_count=int(row.get("post_count") or 0),
        rating="sensitive" if row.get("nsfw") in {"1", "true", "True"} else "safe",
        aliases=sorted(aliases),
        translations=translations,
        wiki=[LocalizedText("zh-CN", wiki)] if wiki else [],
        groups=sorted(groups),
    )


def _read_aliases(path: Path, targets: set[str]) -> dict[str, str]:
    if not path.is_file():
        return {}
    table = parquet.read_table(path)
    columns = set(table.column_names)
    source_column = next(
        (name for name in ("antecedent_name", "alias", "source") if name in columns), None
    )
    target_column = next(
        (name for name in ("consequent_name", "tag", "target") if name in columns), None
    )
    if not source_column or not target_column:
        raise ValueError(f"{path} has unsupported alias columns {sorted(columns)}")
    edges = list(
        zip(
            table[source_column].to_pylist(),
            table[target_column].to_pylist(),
            strict=True,
        )
    )
    clean_edges = [(str(source), str(target)) for source, target in edges]
    sources = {source for source, _ in clean_edges}
    terminal_targets = targets | {
        target for _, target in clean_edges if target not in sources
    }
    resolved = resolve_aliases(clean_edges, terminal_targets)
    return {
        source: target
        for source, target in resolved.items()
        if target in targets
    }


def _read_groups(path: Path) -> tuple[dict[str, list[str]], dict[str, dict[str, Any]]]:
    if not path.is_file():
        return {}, {}
    value = json.loads(path.read_text(encoding="utf-8"))
    tag_to_groups = value.get("tag_to_groups", {})
    groups_to_tags = value.get("group_to_tags") or value.get("groups_to_tags") or {}
    group_names = value.get("group_cn_names", {})
    records: dict[str, dict[str, Any]] = {}
    for group_id, members in groups_to_tags.items():
        clean_id = str(group_id)
        records[clean_id] = {
            "name": str(
                group_names.get(clean_id)
                or clean_id.removeprefix("tag_group:").replace("_", " ")
            ),
            "members": [str(member) for member in members],
        }
    return (
        {str(tag): [str(group) for group in groups] for tag, groups in tag_to_groups.items()},
        records,
    )


def _read_relations(
    path: Path,
    ids: dict[str, int],
    post_counts: dict[str, int],
    total_posts: int,
    relation: str,
) -> list[Relation]:
    if not path.is_file():
        return []
    table = parquet.read_table(path)
    names = table.column_names
    left = next(
        (name for name in ("tag1", "tag_a", "source_tag", "tag") if name in names),
        None,
    )
    right = next(
        (name for name in ("tag2", "tag_b", "target_tag", "artist") if name in names),
        None,
    )
    score = next((name for name in ("npmi", "pmi", "score") if name in names), None)
    count = next((name for name in ("count", "cooc_count") if name in names), None)
    if not left or not right or (not score and not count):
        return []
    result: list[Relation] = []
    scores = table[score].to_pylist() if score else [None] * table.num_rows
    counts = table[count].to_pylist() if count else [None] * table.num_rows
    for source, target, raw_score, pair_count in zip(
        table[left].to_pylist(),
        table[right].to_pylist(),
        scores,
        counts,
        strict=True,
    ):
        if source not in ids or target not in ids:
            continue
        npmi = (
            float(raw_score)
            if raw_score is not None
            else normalized_pmi(
                int(pair_count),
                post_counts[source],
                post_counts[target],
                total_posts,
            )
        )
        if npmi > 0:
            result.append(Relation(ids[source], ids[target], relation, min(npmi, 1.0)))
    result.sort(key=lambda item: (item.source_entity_id, -item.npmi, item.target_entity_id))
    clipped: list[Relation] = []
    per_source: dict[int, int] = defaultdict(int)
    for item in result:
        if per_source[item.source_entity_id] < MAX_RELATIONS_PER_ENTITY:
            clipped.append(item)
            per_source[item.source_entity_id] += 1
    return clipped
