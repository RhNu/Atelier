from __future__ import annotations

import hashlib
import json
import os
import time
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


class OpenAICompatibleEnricher:
    def __init__(self, cache_dir: Path) -> None:
        self.cache_dir = cache_dir
        self.base_url = os.environ.get("OPENAI_BASE_URL", "https://api.openai.com/v1").rstrip("/")
        self.api_key = os.environ.get("OPENAI_API_KEY")
        self.model = os.environ.get("OPENAI_MODEL")
        if not self.api_key or not self.model:
            raise ValueError("OPENAI_API_KEY and OPENAI_MODEL are required for enrichment")

    def enrich(self, canonical_name: str, wiki: str) -> dict[str, Any]:
        request = {
            "model": self.model,
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
                {
                    "role": "system",
                    "content": SYSTEM_PROMPT,
                },
                {
                    "role": "user",
                    "content": json.dumps(
                        {"tag": canonical_name, "wiki": wiki}, ensure_ascii=False
                    ),
                },
            ],
        }
        key = hashlib.sha256(
            json.dumps(request, sort_keys=True, ensure_ascii=False).encode("utf-8")
        ).hexdigest()
        cache_file = self.cache_dir / f"{key}.json"
        if cache_file.is_file():
            return json.loads(cache_file.read_text(encoding="utf-8"))

        headers = {"Authorization": f"Bearer {self.api_key}"}
        last_error: Exception | None = None
        for attempt in range(4):
            try:
                response = httpx.post(
                    f"{self.base_url}/chat/completions",
                    headers=headers,
                    json=request,
                    timeout=90,
                )
                response.raise_for_status()
                content = response.json()["choices"][0]["message"]["content"]
                result = json.loads(content)
                _validate(result)
                write_json(cache_file, result)
                return result
            except (httpx.HTTPError, KeyError, json.JSONDecodeError, ValueError) as error:
                last_error = error
                if attempt < 3:
                    time.sleep(2**attempt)
        raise RuntimeError(f"enrichment failed after retries: {last_error}")

    @property
    def prompt_hash(self) -> str:
        return hashlib.sha256(
            json.dumps(
                {"system": SYSTEM_PROMPT, "schema": OUTPUT_SCHEMA},
                ensure_ascii=False,
                sort_keys=True,
            ).encode("utf-8")
        ).hexdigest()


def enrich_file(input_path: Path, output_path: Path, cache_dir: Path) -> None:
    enricher = OpenAICompatibleEnricher(cache_dir)
    output: list[dict[str, Any]] = []
    for line in input_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        entity = json.loads(line)
        wiki = "\n".join(item["text"] for item in entity.get("wiki", []))
        value = enricher.enrich(entity["canonical_name"], wiki)
        translation = value["chinese_translation"].strip()
        summary = value["chinese_summary"].strip()
        entity["primary_translation"] = translation or entity["primary_translation"]
        entity["translations"] = (
            [{"locale": "zh-CN", "text": translation}] if translation else []
        )
        if summary:
            entity["wiki"] = [{"locale": "zh-CN", "text": summary}]
        entity["expansion_terms"] = sorted(set(value["expansion_terms"]))
        entity["rating"] = value["rating"]
        output.append(entity)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8", newline="\n") as stream:
        for entity in output:
            stream.write(
                json.dumps(entity, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
                + "\n"
            )
    write_json(
        output_path.with_suffix(".provenance.json"),
        {
            "model": enricher.model,
            "prompt_hash": enricher.prompt_hash,
            "entity_count": len(output),
        },
    )


def _validate(value: dict[str, Any]) -> None:
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
