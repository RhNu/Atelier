from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any

import httpx
import numpy as np

from .io import sha256_file, write_json

MODEL_REPOSITORY = "intfloat/multilingual-e5-small"
MODEL_REVISION = "614241f622f53c4eeff9890bdc4f31cfecc418b3"
MODEL_FILE = "onnx/model_qint8_avx512_vnni.onnx"
TOKENIZER_FILE = "onnx/tokenizer.json"
DIMENSIONS = 384
MAX_LENGTH = 128
MIT_LICENSE = """MIT License

Copyright (c) the multilingual-e5-small model contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
"""


def build_semantic_assets(
    entities_path: Path,
    output_dir: Path,
    batch_size: int = 64,
) -> None:
    """Download the pinned model and build resumable normalized fp16 views."""
    try:
        import onnxruntime as ort
        from tokenizers import Tokenizer
    except ImportError as error:
        raise RuntimeError(
            "semantic dependencies are missing; install `.[semantic]`"
        ) from error

    entities = [
        json.loads(line)
        for line in entities_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    entities.sort(key=lambda entity: int(entity["id"]))
    output_dir.mkdir(parents=True, exist_ok=True)
    model_path = output_dir / "model.onnx"
    tokenizer_path = output_dir / "tokenizer.json"
    _download(MODEL_FILE, model_path)
    _download(TOKENIZER_FILE, tokenizer_path)
    (output_dir / "LICENSE-model.txt").write_text(MIT_LICENSE, encoding="utf-8")

    tokenizer = Tokenizer.from_file(str(tokenizer_path))
    tokenizer.enable_truncation(max_length=MAX_LENGTH)
    tokenizer.enable_padding()
    session = ort.InferenceSession(
        str(model_path),
        providers=["CPUExecutionProvider"],
        sess_options=_session_options(ort),
    )
    input_names = {value.name for value in session.get_inputs()}
    output_name = session.get_outputs()[0].name
    identity = _view_map(output_dir / "identity.f16", len(entities))
    knowledge = _view_map(output_dir / "knowledge.f16", len(entities))
    checkpoint_path = output_dir / "semantic.checkpoint.json"
    start = _checkpoint(checkpoint_path, len(entities))

    for offset in range(start, len(entities), batch_size):
        batch = entities[offset : offset + batch_size]
        identity[offset : offset + len(batch)] = _embed(
            session,
            tokenizer,
            [_identity_text(entity) for entity in batch],
            input_names,
        )
        knowledge[offset : offset + len(batch)] = _embed(
            session,
            tokenizer,
            [_knowledge_text(entity) for entity in batch],
            input_names,
        )
        identity.flush()
        knowledge.flush()
        write_json(
            checkpoint_path,
            {"entity_count": len(entities), "completed": offset + len(batch)},
        )

    write_json(
        output_dir / "config.json",
        {
            "dimensions": DIMENSIONS,
            "entity_count": len(entities),
            "max_length": MAX_LENGTH,
            "query_prefix": "query: ",
            "passage_prefix": "passage: ",
            "input_ids": "input_ids",
            "attention_mask": "attention_mask",
            "token_type_ids": (
                "token_type_ids" if "token_type_ids" in input_names else None
            ),
            "output_name": output_name,
            "model": {
                "repository": MODEL_REPOSITORY,
                "revision": MODEL_REVISION,
                "file": MODEL_FILE,
                "license": "MIT",
                "model_sha256": sha256_file(model_path),
                "tokenizer_sha256": sha256_file(tokenizer_path),
            },
        },
    )


def _download(repository_file: str, destination: Path) -> None:
    if destination.is_file():
        return
    url = (
        f"https://huggingface.co/{MODEL_REPOSITORY}/resolve/"
        f"{MODEL_REVISION}/{repository_file}"
    )
    partial = destination.with_suffix(destination.suffix + ".part")
    offset = partial.stat().st_size if partial.is_file() else 0
    headers = {
        "User-Agent": "Atelier-Lexicon-Pipeline/1.0",
        **({"Range": f"bytes={offset}-"} if offset else {}),
    }
    with httpx.stream("GET", url, headers=headers, follow_redirects=True, timeout=120) as response:
        response.raise_for_status()
        mode = "ab" if offset and response.status_code == 206 else "wb"
        with partial.open(mode) as stream:
            for chunk in response.iter_bytes(1024 * 1024):
                stream.write(chunk)
    shutil.move(partial, destination)


def _session_options(ort: Any) -> Any:
    options = ort.SessionOptions()
    options.intra_op_num_threads = 4
    options.inter_op_num_threads = 1
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    return options


def _view_map(path: Path, entity_count: int) -> np.memmap:
    mode = "r+" if path.is_file() else "w+"
    return np.memmap(path, dtype="<f2", mode=mode, shape=(entity_count, DIMENSIONS))


def _checkpoint(path: Path, entity_count: int) -> int:
    if not path.is_file():
        return 0
    value = json.loads(path.read_text(encoding="utf-8"))
    if value.get("entity_count") != entity_count:
        raise ValueError("semantic checkpoint entity count changed")
    return int(value.get("completed", 0))


def _embed(
    session: Any,
    tokenizer: Any,
    texts: list[str],
    input_names: set[str],
) -> np.ndarray:
    encodings = tokenizer.encode_batch(texts)
    input_ids = np.asarray([item.ids for item in encodings], dtype=np.int64)
    attention = np.asarray([item.attention_mask for item in encodings], dtype=np.int64)
    inputs = {"input_ids": input_ids, "attention_mask": attention}
    if "token_type_ids" in input_names:
        inputs["token_type_ids"] = np.asarray(
            [item.type_ids for item in encodings], dtype=np.int64
        )
    hidden = session.run(None, inputs)[0]
    mask = attention[:, :, None].astype(np.float32)
    pooled = (hidden * mask).sum(axis=1) / np.maximum(mask.sum(axis=1), 1)
    pooled /= np.maximum(np.linalg.norm(pooled, axis=1, keepdims=True), 1e-12)
    return pooled.astype("<f2")


def _identity_text(entity: dict[str, Any]) -> str:
    aliases = " ".join(entity.get("aliases", []))
    translations = " ".join(item["text"] for item in entity.get("translations", []))
    groups = " ".join(entity.get("groups", []))
    return (
        f"passage: {entity['canonical_name']}; {aliases}; {translations}; {groups}"
    ).strip()


def _knowledge_text(entity: dict[str, Any]) -> str:
    wiki = " ".join(item["text"] for item in entity.get("wiki", []))
    translations = " ".join(item["text"] for item in entity.get("translations", []))
    return (
        f"passage: {entity['canonical_name']}; {translations}; {wiki}"
    ).strip()
