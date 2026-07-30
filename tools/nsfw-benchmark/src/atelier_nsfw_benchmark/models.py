from __future__ import annotations

import csv
import hashlib
import json
import time
from abc import ABC, abstractmethod
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import numpy as np
import onnxruntime as ort
from huggingface_hub import hf_hub_download
from PIL import Image

PREPROCESS_VERSION = "1"


@dataclass(frozen=True)
class ModelInfo:
    scorer_id: str
    source: str
    revision: str | None
    model_sha256: str
    model_size_bytes: int
    output_columns: list[str]
    preprocess: str


class Scorer(ABC):
    scorer_id: str
    output_columns: tuple[str, ...]

    @property
    @abstractmethod
    def fingerprint(self) -> str: ...

    @property
    @abstractmethod
    def info(self) -> ModelInfo: ...

    @abstractmethod
    def predict(self, image: Image.Image) -> dict[str, float]: ...


def build_session(path: Path, intra_op_threads: int) -> ort.InferenceSession:
    options = ort.SessionOptions()
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    if intra_op_threads > 0:
        options.intra_op_num_threads = intra_op_threads
    return ort.InferenceSession(path, sess_options=options, providers=["CPUExecutionProvider"])


class AnimeClassifierScorer(Scorer):
    def __init__(
        self,
        scorer_id: str,
        repo_id: str,
        revision: str,
        model_name: str,
        prefix: str,
        intra_op_threads: int,
    ):
        self.scorer_id = scorer_id
        self.repo_id = repo_id
        self.revision = revision
        self.model_name = model_name
        self.prefix = prefix
        self.model_path = Path(
            hf_hub_download(
                repo_id=repo_id,
                filename=f"{model_name}/model.onnx",
                revision=revision,
            )
        )
        meta_path = Path(
            hf_hub_download(
                repo_id=repo_id,
                filename=f"{model_name}/meta.json",
                revision=revision,
            )
        )
        self.labels = json.loads(meta_path.read_text(encoding="utf-8"))["labels"]
        self.output_columns = tuple(f"{prefix}_{label}" for label in self.labels)
        self.session = build_session(self.model_path, intra_op_threads)
        self.input = self.session.get_inputs()[0]
        self.output = self.session.get_outputs()[0]
        _, channels, height, width = self.input.shape
        if channels != 3:
            raise ValueError(f"Unexpected input shape for {scorer_id}: {self.input.shape}")
        self.height = height if isinstance(height, int) else 384
        self.width = width if isinstance(width, int) else 384
        self._sha256 = _sha256(self.model_path)

    @property
    def fingerprint(self) -> str:
        return f"{self.revision}:{self._sha256}:anime-square-{PREPROCESS_VERSION}"

    @property
    def info(self) -> ModelInfo:
        return ModelInfo(
            scorer_id=self.scorer_id,
            source=f"https://huggingface.co/{self.repo_id}",
            revision=self.revision,
            model_sha256=self._sha256,
            model_size_bytes=self.model_path.stat().st_size,
            output_columns=list(self.output_columns),
            preprocess="Direct bilinear square resize, RGB CHW, normalize mean/std 0.5",
        )

    def predict(self, image: Image.Image) -> dict[str, float]:
        rgb = _ensure_rgb(image).resize((self.width, self.height), Image.Resampling.BILINEAR)
        array = np.asarray(rgb, dtype=np.float32) / 255.0
        array = ((array - 0.5) / 0.5).transpose(2, 0, 1)[None, ...].astype(np.float32)
        output = self.session.run([self.output.name], {self.input.name: array})[0]
        values = np.asarray(output).reshape(-1)
        return {
            column: float(np.clip(value, 0.0, 1.0))
            for column, value in zip(self.output_columns, values, strict=True)
        }


class WdTaggerScorer(Scorer):
    scorer_id = "wd_tagger"

    def __init__(self, repo_id: str, revision: str, intra_op_threads: int):
        self.repo_id = repo_id
        self.revision = revision
        self.model_path = Path(
            hf_hub_download(repo_id=repo_id, filename="model.onnx", revision=revision)
        )
        tags_path = Path(
            hf_hub_download(repo_id=repo_id, filename="selected_tags.csv", revision=revision)
        )
        with tags_path.open("r", encoding="utf-8", newline="") as file:
            rows = list(csv.DictReader(file))
        self.rating_indices = [index for index, row in enumerate(rows) if int(row["category"]) == 9]
        self.rating_labels = [rows[index]["name"] for index in self.rating_indices]
        self.output_columns = tuple(f"wd_{label}" for label in self.rating_labels)
        self.session = build_session(self.model_path, intra_op_threads)
        self.input = self.session.get_inputs()[0]
        self.output = self.session.get_outputs()[0]
        _, self.height, self.width, channels = self.input.shape
        if channels != 3 or not isinstance(self.height, int) or not isinstance(self.width, int):
            raise ValueError(f"Unexpected WD input shape: {self.input.shape}")
        self._sha256 = _sha256(self.model_path)

    @property
    def fingerprint(self) -> str:
        return f"{self.revision}:{self._sha256}:wd-square-{PREPROCESS_VERSION}"

    @property
    def info(self) -> ModelInfo:
        return ModelInfo(
            scorer_id=self.scorer_id,
            source=f"https://huggingface.co/{self.repo_id}",
            revision=self.revision,
            model_sha256=self._sha256,
            model_size_bytes=self.model_path.stat().st_size,
            output_columns=list(self.output_columns),
            preprocess="White square pad, bicubic resize, NHWC BGR float32 0..255",
        )

    def predict(self, image: Image.Image) -> dict[str, float]:
        rgb = _ensure_rgb(image)
        padded = _letterbox_square(rgb).resize((self.width, self.height), Image.Resampling.BICUBIC)
        array = np.asarray(padded, dtype=np.float32)[..., ::-1][None, ...]
        output = self.session.run([self.output.name], {self.input.name: array})[0]
        values = np.asarray(output).reshape(-1)
        return {
            column: float(np.clip(values[index], 0.0, 1.0))
            for column, index in zip(self.output_columns, self.rating_indices, strict=True)
        }


def timed_prediction(scorer: Scorer, image: Image.Image) -> tuple[dict[str, float], float]:
    started = time.perf_counter()
    scores = scorer.predict(image)
    return scores, (time.perf_counter() - started) * 1000.0


def model_info_dict(info: ModelInfo) -> dict[str, Any]:
    return asdict(info)


def _ensure_rgb(image: Image.Image) -> Image.Image:
    if image.mode == "RGBA":
        background = Image.new("RGBA", image.size, (255, 255, 255, 255))
        background.alpha_composite(image)
        return background.convert("RGB")
    if image.mode != "RGB":
        return image.convert("RGB")
    return image


def _letterbox_square(image: Image.Image) -> Image.Image:
    side = max(image.size)
    canvas = Image.new("RGB", (side, side), (255, 255, 255))
    canvas.paste(image, ((side - image.width) // 2, (side - image.height) // 2))
    return canvas


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()
