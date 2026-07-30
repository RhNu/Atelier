from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp"}
GROUP_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}")


@dataclass(frozen=True)
class Sample:
    path: Path
    relative_path: str
    label: int
    label_name: str
    group: str
    sha256: str
    width: int
    height: int


@dataclass(frozen=True)
class DatasetSummary:
    samples: list[Sample]
    exact_duplicate_sets: int


def discover_dataset(root: Path) -> DatasetSummary:
    root = root.resolve()
    if not root.is_dir():
        raise FileNotFoundError(f"Dataset directory does not exist: {root}")

    samples: list[Sample] = []
    hashes: dict[str, list[Sample]] = {}
    for label_name, label in (("sfw", 0), ("nsfw", 1)):
        label_dir = root / label_name
        if not label_dir.is_dir():
            raise FileNotFoundError(f"Missing label directory: {label_dir}")
        for path in sorted(label_dir.rglob("*")):
            if not path.is_file() or path.suffix.lower() not in IMAGE_EXTENSIONS:
                continue
            digest = _sha256(path)
            with Image.open(path) as image:
                width, height = image.size
                image.verify()
            stem = path.stem
            match = GROUP_PATTERN.match(stem)
            group = match.group(0) if match else stem[:10]
            sample = Sample(
                path=path.resolve(),
                relative_path=path.relative_to(root).as_posix(),
                label=label,
                label_name=label_name,
                group=group,
                sha256=digest,
                width=width,
                height=height,
            )
            samples.append(sample)
            hashes.setdefault(digest, []).append(sample)

    if not samples:
        raise ValueError(f"No supported images found below {root}")

    cross_label = [
        duplicates
        for duplicates in hashes.values()
        if len({sample.label for sample in duplicates}) > 1
    ]
    if cross_label:
        paths = ", ".join(sample.relative_path for sample in cross_label[0])
        raise ValueError(f"Identical image has conflicting labels: {paths}")

    return DatasetSummary(
        samples=samples,
        exact_duplicate_sets=sum(len(items) > 1 for items in hashes.values()),
    )


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()
