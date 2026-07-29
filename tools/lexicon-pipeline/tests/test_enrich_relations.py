import json
from pathlib import Path

import httpx
import pytest

from atelier_lexicon.enrich import (
    OpenAICompatibleBatchEnricher,
    _merge_enrichment,
    _partition_requests,
    _validate,
)
from atelier_lexicon.relations import normalized_pmi
from atelier_lexicon.semantic import _checkpoint


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


def test_enrichment_merges_instead_of_discarding_existing_metadata() -> None:
    entity = {
        "id": 1,
        "primary_translation": "旧译名",
        "translations": [{"locale": "en", "text": "existing"}],
        "wiki": [{"locale": "en", "text": "Original wiki"}],
        "expansion_terms": ["existing term"],
        "rating": "sensitive",
    }
    merged = _merge_enrichment(
        entity,
        {
            "chinese_translation": "新译名",
            "chinese_summary": "中文摘要",
            "expansion_terms": ["new term", "existing term"],
            "rating": "safe",
        },
    )
    assert merged["primary_translation"] == "新译名"
    assert merged["translations"] == [
        {"locale": "zh-CN", "text": "新译名"},
        {"locale": "en", "text": "existing"},
    ]
    assert len(merged["wiki"]) == 2
    assert merged["expansion_terms"] == ["existing term", "new term"]
    assert merged["rating"] == "sensitive"


def test_batch_enrichment_uploads_once_and_reuses_result_cache(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("OPENAI_API_KEY", "test-key")
    monkeypatch.setenv("OPENAI_MODEL", "test-model")
    calls: list[str] = []
    output_rows = [
        _batch_output("entity-1", "红发"),
        _batch_output("entity-2", "蓝发"),
    ]

    def handler(request: httpx.Request) -> httpx.Response:
        calls.append(f"{request.method} {request.url.path}")
        if request.url.path == "/v1/files":
            assert b'purpose"\r\n\r\nbatch' in request.read()
            if calls.count("POST /v1/files") == 1:
                return httpx.Response(500, text="retry")
            return httpx.Response(200, json={"id": "file-input"})
        if request.url.path == "/v1/batches":
            body = json.loads(request.read())
            assert body["endpoint"] == "/v1/chat/completions"
            assert body["completion_window"] == "24h"
            return httpx.Response(
                200,
                json={
                    "id": "batch-1",
                    "status": "completed",
                    "output_file_id": "file-output",
                    "error_file_id": None,
                },
            )
        if request.url.path == "/v1/files/file-output/content":
            return httpx.Response(
                200,
                text="\n".join(json.dumps(row) for row in output_rows),
            )
        raise AssertionError(f"unexpected request {request.url}")

    client = httpx.Client(transport=httpx.MockTransport(handler))
    enricher = OpenAICompatibleBatchEnricher(
        tmp_path,
        batch_size=50,
        poll_seconds=0,
        client=client,
        sleep=lambda _: None,
    )
    entities = [
        {"id": 1, "canonical_name": "red_hair", "wiki": []},
        {"id": 2, "canonical_name": "blue_hair", "wiki": []},
    ]
    first = enricher.enrich_entities(entities)
    second = enricher.enrich_entities(entities)
    assert first == second
    assert first[1]["chinese_translation"] == "红发"
    assert calls == [
        "POST /v1/files",
        "POST /v1/files",
        "POST /v1/batches",
        "GET /v1/files/file-output/content",
    ]


def test_batch_partition_respects_configured_request_count() -> None:
    requests = [
        {
            "custom_id": f"entity-{index}",
            "body": {"model": "test", "messages": []},
        }
        for index in range(5)
    ]
    assert [len(chunk) for chunk in _partition_requests(requests, 2)] == [2, 2, 1]


def test_semantic_checkpoint_is_bound_to_exact_enrichment_input(
    tmp_path: Path,
) -> None:
    checkpoint = tmp_path / "semantic.checkpoint.json"
    checkpoint.write_text(
        json.dumps(
            {
                "entity_count": 2,
                "entities_sha256": "a" * 64,
                "completed": 1,
            }
        ),
        encoding="utf-8",
    )
    assert _checkpoint(checkpoint, 2, "a" * 64) == 1
    with pytest.raises(ValueError, match="different entities"):
        _checkpoint(checkpoint, 2, "b" * 64)


def _batch_output(custom_id: str, translation: str) -> dict[str, object]:
    value = {
        "chinese_translation": translation,
        "chinese_summary": f"{translation}摘要",
        "expansion_terms": [],
        "rating": "safe",
    }
    return {
        "custom_id": custom_id,
        "error": None,
        "response": {
            "status_code": 200,
            "body": {
                "choices": [
                    {"message": {"content": json.dumps(value, ensure_ascii=False)}}
                ]
            },
        },
    }
