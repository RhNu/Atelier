from __future__ import annotations

import json
import csv
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import httpx

from .io import read_string_set, write_json, write_jsonl

DANBOORU_TAGS_URL = "https://danbooru.donmai.us/tags.json"
SUPPORTED_CATEGORIES = {0, 1, 3, 4}
SELECTED_BATCH_SIZE = 100


def fetch_danbooru_tags(output_dir: Path, max_pages: int | None = None) -> None:
    """Fetch a resumable, ID-bearing Danbooru tag snapshot."""
    output_dir.mkdir(parents=True, exist_ok=True)
    records_path = output_dir / "tags.jsonl"
    checkpoint_path = output_dir / "tags.checkpoint.json"
    checkpoint = _read_checkpoint(checkpoint_path)
    before_id = checkpoint.get("before_id")
    page_count = int(checkpoint.get("page_count", 0))
    fetched = int(checkpoint.get("fetched", 0))
    mode = "a" if records_path.is_file() and before_id else "w"

    with (
        httpx.Client(
            timeout=60,
            follow_redirects=True,
            headers={"User-Agent": "Atelier-Lexicon-Pipeline/1.0"},
        ) as client,
        records_path.open(mode, encoding="utf-8", newline="\n") as stream,
    ):
        while max_pages is None or page_count < max_pages:
            params: dict[str, str | int] = {"limit": 1000}
            if before_id:
                params["page"] = f"b{before_id}"
            response = client.get(DANBOORU_TAGS_URL, params=params)
            response.raise_for_status()
            rows = response.json()
            if not isinstance(rows, list) or not rows:
                break
            valid_rows = [_validate_tag(row) for row in rows]
            for row in valid_rows:
                if row["category"] in SUPPORTED_CATEGORIES:
                    stream.write(
                        json.dumps(
                            row,
                            ensure_ascii=False,
                            sort_keys=True,
                            separators=(",", ":"),
                        )
                    )
                    stream.write("\n")
                    fetched += 1
            stream.flush()
            before_id = min(int(row["id"]) for row in valid_rows)
            page_count += 1
            write_json(
                checkpoint_path,
                {
                    "before_id": before_id,
                    "fetched": fetched,
                    "page_count": page_count,
                },
            )

    write_json(
        output_dir / "danbooru-tags-source.json",
        {
            "id": "danbooru-tags-api",
            "url": DANBOORU_TAGS_URL,
            "retrieved_at": datetime.now(timezone.utc).isoformat(),
            "last_before_id": before_id,
            "page_count": page_count,
            "record_count": fetched,
        },
    )


def fetch_selected_danbooru_tags(
    names_from: Path,
    output_dir: Path,
    deny_file: Path,
) -> None:
    """Resolve the curated source vocabulary through batched exact-name API queries."""
    names = _read_names(names_from)
    batches = [
        names[offset : offset + SELECTED_BATCH_SIZE]
        for offset in range(0, len(names), SELECTED_BATCH_SIZE)
    ]
    cache_dir = output_dir / f"tag-batches-{SELECTED_BATCH_SIZE}"
    cache_dir.mkdir(parents=True, exist_ok=True)
    pending = [
        (index, batch)
        for index, batch in enumerate(batches)
        if not (cache_dir / f"{index:05}.json").is_file()
    ]
    with (
        httpx.Client(
            timeout=60,
            follow_redirects=True,
            headers={"User-Agent": "Atelier-Lexicon-Pipeline/1.0"},
        ) as client,
        ThreadPoolExecutor(max_workers=4) as executor,
    ):
        futures = [
            executor.submit(_fetch_exact_batch, client, index, batch, cache_dir)
            for index, batch in pending
        ]
        for future in futures:
            future.result()

    rows: list[dict[str, Any]] = []
    for index in range(len(batches)):
        value = json.loads((cache_dir / f"{index:05}.json").read_text(encoding="utf-8"))
        rows.extend(value)
    by_name = {str(row["name"]): row for row in rows}
    missing = sorted(set(names).difference(by_name).difference(read_string_set(deny_file)))
    if missing:
        write_json(output_dir / "tags-missing.json", missing)
        raise ValueError(
            f"Danbooru did not resolve {len(missing)} requested tags; "
            f"see {output_dir / 'tags-missing.json'}"
        )
    ordered = sorted(by_name.values(), key=lambda row: int(row["id"]))
    write_jsonl(output_dir / "tags.jsonl", ordered)
    write_json(
        output_dir / "danbooru-tags-source.json",
        {
            "id": "danbooru-tags-api",
            "url": DANBOORU_TAGS_URL,
            "retrieved_at": datetime.now(timezone.utc).isoformat(),
            "query": "exact curated names",
            "record_count": len(ordered),
            "batch_count": len(batches),
        },
    )


def _fetch_exact_batch(
    client: httpx.Client,
    index: int,
    names: list[str],
    cache_dir: Path,
) -> None:
    last_error: Exception | None = None
    for attempt in range(4):
        try:
            response = client.get(
                DANBOORU_TAGS_URL,
                params={
                    "limit": 1000,
                    "search[name_comma]": ",".join(names),
                },
            )
            response.raise_for_status()
            rows = [_validate_tag(row) for row in response.json()]
            write_json(cache_dir / f"{index:05}.json", rows)
            return
        except (httpx.HTTPError, ValueError, TypeError) as error:
            last_error = error
            if attempt < 3:
                time.sleep(2**attempt)
    raise RuntimeError(f"Danbooru tag batch {index} failed: {last_error}")


def _read_names(path: Path) -> list[str]:
    encoding = "utf-8-sig"
    try:
        path.read_text(encoding=encoding)
    except UnicodeDecodeError:
        encoding = "gb18030"
    with path.open("r", encoding=encoding, newline="") as stream:
        names = {
            row["name"]
            for row in csv.DictReader(stream)
            if row.get("name") and row.get("category") in {"0", "1", "3", "4"}
        }
    return sorted(names)


def _read_checkpoint(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def _validate_tag(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("Danbooru returned a non-object tag record")
    required = {"id", "name", "category", "post_count"}
    missing = required.difference(value)
    if missing:
        raise ValueError(f"Danbooru tag record is missing {sorted(missing)}")
    return {
        "id": int(value["id"]),
        "name": str(value["name"]),
        "category": int(value["category"]),
        "post_count": int(value["post_count"]),
        "is_deleted": bool(value.get("is_deleted", False)),
        "created_at": value.get("created_at"),
        "updated_at": value.get("updated_at"),
    }
