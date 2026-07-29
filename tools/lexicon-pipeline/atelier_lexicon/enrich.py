from __future__ import annotations

import hashlib
import json
import os
import time
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import Any

import httpx

from .io import write_json

SYSTEM_PROMPT = (
    "Return Chinese localization metadata for one Danbooru tag. "
    "Follow the supplied JSON schema exactly."
)
OUTPUT_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": [
        "chinese_translation",
        "chinese_summary",
        "expansion_terms",
        "rating",
    ],
    "properties": {
        "chinese_translation": {"type": "string"},
        "chinese_summary": {"type": "string"},
        "expansion_terms": {"type": "array", "items": {"type": "string"}},
        "rating": {"type": "string", "enum": ["safe", "sensitive", "unknown"]},
    },
}

BATCH_ENDPOINT = "/v1/chat/completions"
DEFAULT_BATCH_SIZE = 1_000
MAX_BATCH_REQUESTS = 50_000
MAX_BATCH_BYTES = 190 * 1024 * 1024
TERMINAL_BATCH_STATES = {"completed", "failed", "expired", "cancelled"}


class OpenAICompatibleBatchEnricher:
    """Submit resumable, content-addressed enrichment work through Batch API."""

    def __init__(
        self,
        cache_dir: Path,
        batch_size: int = DEFAULT_BATCH_SIZE,
        poll_seconds: float = 30,
        max_wait_seconds: float = 90_000,
        client: httpx.Client | None = None,
        sleep: Callable[[float], None] = time.sleep,
    ) -> None:
        self.cache_dir = cache_dir
        self.base_url = os.environ.get(
            "OPENAI_BASE_URL", "https://api.openai.com/v1"
        ).rstrip("/")
        self.api_key = os.environ.get("OPENAI_API_KEY")
        self.model = os.environ.get("OPENAI_MODEL")
        if not self.api_key or not self.model:
            raise ValueError(
                "OPENAI_API_KEY and OPENAI_MODEL are required for enrichment"
            )
        if not 1 <= batch_size <= MAX_BATCH_REQUESTS:
            raise ValueError(
                f"batch_size must be between 1 and {MAX_BATCH_REQUESTS}"
            )
        if poll_seconds < 0 or max_wait_seconds <= 0:
            raise ValueError(
                "poll duration must be non-negative and maximum wait must be positive"
            )
        self.batch_size = batch_size
        self.poll_seconds = poll_seconds
        self.max_wait_seconds = max_wait_seconds
        self.sleep = sleep
        self.client = client or httpx.Client(timeout=120, follow_redirects=True)
        self._owns_client = client is None
        self.result_dir = cache_dir / "results"
        self.batch_dir = cache_dir / "batches"
        self.result_dir.mkdir(parents=True, exist_ok=True)
        self.batch_dir.mkdir(parents=True, exist_ok=True)

    def close(self) -> None:
        if self._owns_client:
            self.client.close()

    def enrich_entities(
        self, entities: list[dict[str, Any]]
    ) -> dict[int, dict[str, Any]]:
        results: dict[int, dict[str, Any]] = {}
        pending: list[dict[str, Any]] = []
        for entity in entities:
            entity_id = int(entity["id"])
            body = _request_body(
                str(self.model),
                str(entity["canonical_name"]),
                _wiki_text(entity),
            )
            cache_key = _request_hash(body)
            cache_file = self.result_dir / f"{cache_key}.json"
            if cache_file.is_file():
                value = json.loads(cache_file.read_text(encoding="utf-8"))
                _validate(value)
                results[entity_id] = value
                continue
            pending.append(
                {
                    "entity_id": entity_id,
                    "custom_id": f"entity-{entity_id}",
                    "cache_key": cache_key,
                    "body": body,
                }
            )

        for chunk in _partition_requests(pending, self.batch_size):
            results.update(self._run_batch(chunk))
        return results

    def _run_batch(
        self, requests: list[dict[str, Any]]
    ) -> dict[int, dict[str, Any]]:
        input_bytes = b"".join(_batch_line(item) for item in requests)
        input_hash = hashlib.sha256(input_bytes).hexdigest()
        input_path = self.batch_dir / f"{input_hash}.jsonl"
        state_path = self.batch_dir / f"{input_hash}.state.json"
        if input_path.is_file() and hashlib.sha256(input_path.read_bytes()).hexdigest() != input_hash:
            raise ValueError(f"cached batch input is corrupt: {input_path}")
        if not input_path.is_file():
            input_path.write_bytes(input_bytes)
        state = _read_state(state_path)
        if (
            state.get("status") in TERMINAL_BATCH_STATES
            and state.get("status") != "completed"
            and not state.get("output_file_id")
        ):
            failed_path = self.batch_dir / (
                f"{input_hash}.{state.get('batch_id', 'unknown')}.failed.json"
            )
            state_path.replace(failed_path)
            state = {}
        if "batch_id" not in state:
            input_file_id = self._upload_batch_file(input_path)
            batch = self._request_json(
                "POST",
                "/batches",
                json={
                    "input_file_id": input_file_id,
                    "endpoint": BATCH_ENDPOINT,
                    "completion_window": "24h",
                    "metadata": {
                        "pipeline": "atelier-lexicon",
                        "prompt_hash": self.prompt_hash[:32],
                    },
                },
            )
            state = {
                "input_hash": input_hash,
                "input_file_id": input_file_id,
                "batch_id": str(batch["id"]),
                "status": str(batch["status"]),
                "output_file_id": batch.get("output_file_id"),
                "error_file_id": batch.get("error_file_id"),
            }
            write_json(state_path, state)

        batch = self._wait_for_batch(str(state["batch_id"]), state_path, state)
        completed = self._collect_output(batch, requests, input_hash)
        if len(completed) != len(requests):
            retry_path = self.batch_dir / (
                f"{input_hash}.{batch.get('id', batch.get('batch_id'))}.retry.json"
            )
            if state_path.is_file():
                state_path.replace(retry_path)
            missing = sorted(
                int(item["entity_id"])
                for item in requests
                if int(item["entity_id"]) not in completed
            )
            raise RuntimeError(
                f"batch {batch['id']} completed {len(completed)}/{len(requests)} "
                f"requests; rerun to retry missing entity IDs {missing[:10]}"
            )
        return completed

    def _upload_batch_file(self, path: Path) -> str:
        last_error: Exception | None = None
        for attempt in range(4):
            try:
                with path.open("rb") as stream:
                    response = self.client.post(
                        f"{self.base_url}/files",
                        headers=self._authorization_headers(),
                        data={"purpose": "batch"},
                        files={"file": (path.name, stream, "application/jsonl")},
                    )
                response.raise_for_status()
                return str(response.json()["id"])
            except (httpx.HTTPError, KeyError, TypeError) as error:
                last_error = error
                if attempt < 3:
                    self.sleep(2**attempt)
        raise RuntimeError(f"batch file upload failed after retries: {last_error}")

    def _wait_for_batch(
        self, batch_id: str, state_path: Path, state: dict[str, Any]
    ) -> dict[str, Any]:
        started = time.monotonic()
        batch = state
        while str(batch.get("status")) not in TERMINAL_BATCH_STATES:
            if time.monotonic() - started > self.max_wait_seconds:
                raise TimeoutError(
                    f"batch {batch_id} is still {batch.get('status')}; "
                    "its ID is cached and the next run will resume polling"
                )
            if self.poll_seconds:
                self.sleep(self.poll_seconds)
            batch = self._request_json("GET", f"/batches/{batch_id}")
            write_json(
                state_path,
                {
                    "input_hash": state["input_hash"],
                    "input_file_id": state["input_file_id"],
                    "batch_id": batch_id,
                    "status": batch["status"],
                    "output_file_id": batch.get("output_file_id"),
                    "error_file_id": batch.get("error_file_id"),
                },
            )
        return batch

    def _collect_output(
        self,
        batch: dict[str, Any],
        requests: list[dict[str, Any]],
        input_hash: str,
    ) -> dict[int, dict[str, Any]]:
        if error_file_id := batch.get("error_file_id"):
            error_content = self._request(
                "GET", f"/files/{error_file_id}/content"
            ).content
            (self.batch_dir / f"{input_hash}.errors.jsonl").write_bytes(
                error_content
            )
        output_file_id = batch.get("output_file_id")
        if not output_file_id:
            raise RuntimeError(
                f"batch {batch.get('id', batch.get('batch_id'))} ended as "
                f"{batch['status']} without output"
            )
        output = self._request(
            "GET", f"/files/{output_file_id}/content"
        ).text
        by_custom_id = {str(item["custom_id"]): item for item in requests}
        completed: dict[int, dict[str, Any]] = {}
        for line_number, line in enumerate(output.splitlines(), 1):
            if not line.strip():
                continue
            row = json.loads(line)
            custom_id = str(row["custom_id"])
            request = by_custom_id.get(custom_id)
            if request is None:
                raise ValueError(
                    f"batch output line {line_number} has unknown custom_id {custom_id}"
                )
            response = row.get("response")
            if row.get("error") or not response or response.get("status_code") != 200:
                continue
            value = _response_value(response["body"])
            _validate(value)
            cache_file = self.result_dir / f"{request['cache_key']}.json"
            write_json(cache_file, value)
            completed[int(request["entity_id"])] = value
        return completed

    def _request_json(
        self, method: str, path: str, **kwargs: Any
    ) -> dict[str, Any]:
        value = self._request(method, path, **kwargs).json()
        if not isinstance(value, dict):
            raise ValueError(f"{path} returned a non-object response")
        return value

    def _request(self, method: str, path: str, **kwargs: Any) -> httpx.Response:
        last_error: Exception | None = None
        headers = {
            **self._authorization_headers(),
            **kwargs.pop("headers", {}),
        }
        for attempt in range(4):
            try:
                response = self.client.request(
                    method,
                    f"{self.base_url}{path}",
                    headers=headers,
                    **kwargs,
                )
                response.raise_for_status()
                return response
            except httpx.HTTPError as error:
                last_error = error
                if attempt < 3:
                    self.sleep(2**attempt)
        raise RuntimeError(f"{method} {path} failed after retries: {last_error}")

    def _authorization_headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self.api_key}"}

    @property
    def prompt_hash(self) -> str:
        return hashlib.sha256(
            json.dumps(
                {"system": SYSTEM_PROMPT, "schema": OUTPUT_SCHEMA},
                ensure_ascii=False,
                sort_keys=True,
            ).encode("utf-8")
        ).hexdigest()


def enrich_file(
    input_path: Path,
    output_path: Path,
    cache_dir: Path,
    batch_size: int = DEFAULT_BATCH_SIZE,
    poll_seconds: float = 30,
    max_wait_seconds: float = 90_000,
) -> None:
    entities = [
        json.loads(line)
        for line in input_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    enricher = OpenAICompatibleBatchEnricher(
        cache_dir,
        batch_size=batch_size,
        poll_seconds=poll_seconds,
        max_wait_seconds=max_wait_seconds,
    )
    try:
        values = enricher.enrich_entities(entities)
        enriched = [
            _merge_enrichment(entity, values[int(entity["id"])])
            for entity in entities
        ]
        _write_jsonl_atomic(output_path, enriched)
        write_json(
            output_path.with_suffix(".provenance.json"),
            {
                "mode": "batch",
                "endpoint": BATCH_ENDPOINT,
                "model": enricher.model,
                "prompt_hash": enricher.prompt_hash,
                "entity_count": len(enriched),
                "input_sha256": _sha256_file(input_path),
                "output_sha256": _sha256_file(output_path),
            },
        )
    finally:
        enricher.close()


def _request_body(model: str, canonical_name: str, wiki: str) -> dict[str, Any]:
    return {
        "model": model,
        "temperature": 0,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "danbooru_tag_enrichment",
                "strict": True,
                "schema": OUTPUT_SCHEMA,
            },
        },
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {
                "role": "user",
                "content": json.dumps(
                    {"tag": canonical_name, "wiki": wiki},
                    ensure_ascii=False,
                ),
            },
        ],
    }


def _request_hash(body: dict[str, Any]) -> str:
    return hashlib.sha256(
        json.dumps(
            body,
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode("utf-8")
    ).hexdigest()


def _batch_line(request: dict[str, Any]) -> bytes:
    value = {
        "custom_id": request["custom_id"],
        "method": "POST",
        "url": BATCH_ENDPOINT,
        "body": request["body"],
    }
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def _partition_requests(
    requests: Iterable[dict[str, Any]], batch_size: int
) -> list[list[dict[str, Any]]]:
    chunks: list[list[dict[str, Any]]] = []
    current: list[dict[str, Any]] = []
    current_bytes = 0
    for request in requests:
        size = len(_batch_line(request))
        if size > MAX_BATCH_BYTES:
            raise ValueError(f"request {request['custom_id']} exceeds Batch API size limit")
        if current and (
            len(current) >= batch_size or current_bytes + size > MAX_BATCH_BYTES
        ):
            chunks.append(current)
            current = []
            current_bytes = 0
        current.append(request)
        current_bytes += size
    if current:
        chunks.append(current)
    return chunks


def _response_value(body: dict[str, Any]) -> dict[str, Any]:
    content = body["choices"][0]["message"]["content"]
    if not isinstance(content, str):
        raise ValueError("batch completion content must be a JSON string")
    value = json.loads(content)
    if not isinstance(value, dict):
        raise ValueError("batch completion content must decode to an object")
    return value


def _merge_enrichment(
    entity: dict[str, Any], value: dict[str, Any]
) -> dict[str, Any]:
    merged = dict(entity)
    translation = value["chinese_translation"].strip()
    summary = value["chinese_summary"].strip()
    if translation:
        merged["primary_translation"] = translation
        merged["translations"] = _merge_localized(
            entity.get("translations", []), "zh-CN", translation
        )
    if summary:
        merged["wiki"] = _merge_localized(
            entity.get("wiki", []), "zh-CN", summary
        )
    merged["expansion_terms"] = sorted(
        {
            *entity.get("expansion_terms", []),
            *(item.strip() for item in value["expansion_terms"] if item.strip()),
        }
    )
    merged["rating"] = (
        "sensitive" if entity.get("rating") == "sensitive" else value["rating"]
    )
    return merged


def _merge_localized(
    existing: list[dict[str, Any]], locale: str, text: str
) -> list[dict[str, str]]:
    result = [
        {"locale": str(item["locale"]), "text": str(item["text"])}
        for item in existing
    ]
    candidate = {"locale": locale, "text": text}
    if candidate not in result:
        result.insert(0, candidate)
    return result


def _wiki_text(entity: dict[str, Any]) -> str:
    return "\n".join(str(item["text"]) for item in entity.get("wiki", []))


def _read_state(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def _write_jsonl_atomic(path: Path, entities: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="\n") as stream:
        for entity in entities:
            stream.write(
                json.dumps(
                    entity,
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                )
                + "\n"
            )
    temporary.replace(path)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _validate(value: dict[str, Any]) -> None:
    expected_keys = set(OUTPUT_SCHEMA["properties"])
    if set(value) != expected_keys:
        raise ValueError(
            f"enrichment fields must be exactly {sorted(expected_keys)}"
        )
    if value.get("rating") not in {"safe", "sensitive", "unknown"}:
        raise ValueError("rating must be safe, sensitive, or unknown")
    if not isinstance(value.get("chinese_translation"), str):
        raise ValueError("chinese_translation must be a string")
    if not isinstance(value.get("chinese_summary"), str):
        raise ValueError("chinese_summary must be a string")
    if not isinstance(value.get("expansion_terms"), list) or not all(
        isinstance(item, str) for item in value["expansion_terms"]
    ):
        raise ValueError("expansion_terms must be a string array")
