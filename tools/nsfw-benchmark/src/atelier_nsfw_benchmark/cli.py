from __future__ import annotations

import argparse
import csv
import gc
import sys
import tomllib
from pathlib import Path
from typing import Any

from PIL import Image

from .dataset import DatasetSummary, discover_dataset
from .evaluation import evaluate_all, rank_results
from .models import (
    AnimeClassifierScorer,
    Scorer,
    WdTaggerScorer,
    model_info_dict,
    timed_prediction,
)
from .report import write_reports


def main() -> None:
    parser = argparse.ArgumentParser(description="Benchmark NSFW models on local labeled images.")
    parser.add_argument(
        "--config",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "benchmark.toml",
    )
    parser.add_argument(
        "--dataset",
        type=Path,
        default=Path(__file__).resolve().parents[4] / "temp" / "res-nsfw-benchmark",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parents[4] / "temp" / "nsfw-benchmark-results",
    )
    parser.add_argument(
        "--skip-model",
        action="append",
        default=[],
        choices=["anime_dbrating", "anime_rating", "wd_tagger"],
    )
    args = parser.parse_args()
    run(args.config.resolve(), args.dataset.resolve(), args.output.resolve(), set(args.skip_model))


def run(config_path: Path, dataset_path: Path, output_dir: Path, skipped: set[str]) -> None:
    config = tomllib.loads(config_path.read_text(encoding="utf-8"))
    dataset = discover_dataset(dataset_path)
    output_dir.mkdir(parents=True, exist_ok=True)
    _print_dataset(dataset)

    runtime = config.get("runtime", {})
    threads = int(runtime.get("intra_op_threads", 0))
    raw_dir = output_dir / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    model_info: list[dict[str, Any]] = []
    score_parts: list[dict[str, dict[str, str]]] = []

    for scorer in _scorers(config["models"], threads, skipped):
        size_mb = scorer.info.model_size_bytes / 1_000_000
        print(f"\n[{scorer.scorer_id}] model ready: {size_mb:.1f} MB")
        part = _run_scorer(scorer, dataset, raw_dir / f"{scorer.scorer_id}.csv")
        score_parts.append(part)
        model_info.append(model_info_dict(scorer.info))
        del scorer
        gc.collect()

    if not score_parts:
        raise ValueError("No models are enabled")
    rows = _combine_scores(dataset, score_parts)
    _write_csv(output_dir / "scores.csv", rows)

    benchmark = config["benchmark"]
    model_sizes = {info["scorer_id"]: int(info["model_size_bytes"]) for info in model_info}
    results = evaluate_all(
        rows=rows,
        recall_target=float(benchmark["recall_target"]),
        folds=int(benchmark["folds"]),
        random_seed=int(benchmark["random_seed"]),
        model_sizes=model_sizes,
    )
    selected = rank_results(
        results,
        compact_limit_mb=float(benchmark["compact_limit_mb"]),
        recall_target=float(benchmark["recall_target"]),
    )
    write_reports(
        output_dir=output_dir,
        dataset=dataset,
        rows=rows,
        results=results,
        selected=selected,
        model_info=model_info,
        config_snapshot=benchmark,
    )
    print(f"\nOverall: {selected['overall']}")
    print(f"Compact: {selected['compact']}")
    print(f"Report: {output_dir / 'REPORT.md'}")


def _scorers(
    configs: dict[str, dict[str, Any]],
    threads: int,
    skipped: set[str],
):
    for scorer_id, values in configs.items():
        if not values.get("enabled", True) or scorer_id in skipped:
            continue
        if scorer_id in {"anime_dbrating", "anime_rating"}:
            yield AnimeClassifierScorer(
                scorer_id=scorer_id,
                repo_id=values["repo_id"],
                revision=values["revision"],
                model_name=values["model_name"],
                prefix="dbrating" if scorer_id == "anime_dbrating" else "rating",
                intra_op_threads=threads,
            )
        elif scorer_id == "wd_tagger":
            yield WdTaggerScorer(
                repo_id=values["repo_id"],
                revision=values["revision"],
                intra_op_threads=threads,
            )
        else:
            raise ValueError(f"Unknown scorer: {scorer_id}")


def _run_scorer(
    scorer: Scorer,
    dataset: DatasetSummary,
    cache_path: Path,
) -> dict[str, dict[str, str]]:
    cached = _load_cache(cache_path, scorer.fingerprint)
    pending = [sample for sample in dataset.samples if sample.sha256 not in cached]
    fieldnames = [
        "relative_path",
        "sha256",
        "fingerprint",
        *scorer.output_columns,
        f"{scorer.scorer_id}_latency_ms",
    ]
    with cache_path.open("a", encoding="utf-8", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=fieldnames)
        if cache_path.stat().st_size == 0:
            writer.writeheader()
        for index, sample in enumerate(pending, start=1):
            with Image.open(sample.path) as image:
                scores, latency_ms = timed_prediction(scorer, image)
            row = {
                "relative_path": sample.relative_path,
                "sha256": sample.sha256,
                "fingerprint": scorer.fingerprint,
                **scores,
                f"{scorer.scorer_id}_latency_ms": latency_ms,
            }
            writer.writerow(row)
            file.flush()
            cached[sample.sha256] = {key: str(value) for key, value in row.items()}
            if index == 1 or index % 25 == 0 or index == len(pending):
                print(f"[{scorer.scorer_id}] {index}/{len(pending)} new images")
    return cached


def _load_cache(path: Path, fingerprint: str) -> dict[str, dict[str, str]]:
    if not path.exists():
        return {}
    with path.open("r", encoding="utf-8", newline="") as file:
        return {
            row["sha256"]: row
            for row in csv.DictReader(file)
            if row.get("fingerprint") == fingerprint
        }


def _combine_scores(
    dataset: DatasetSummary,
    parts: list[dict[str, dict[str, str]]],
) -> list[dict[str, str]]:
    rows = []
    for sample in dataset.samples:
        row = {
            "relative_path": sample.relative_path,
            "sha256": sample.sha256,
            "label": str(sample.label),
            "label_name": sample.label_name,
            "group": sample.group,
            "width": str(sample.width),
            "height": str(sample.height),
        }
        for part in parts:
            cached = part[sample.sha256]
            row.update(
                {
                    key: value
                    for key, value in cached.items()
                    if key not in {"relative_path", "sha256", "fingerprint"}
                }
            )
        rows.append(row)
    return rows


def _print_dataset(dataset: DatasetSummary) -> None:
    sfw = sum(sample.label == 0 for sample in dataset.samples)
    nsfw = sum(sample.label == 1 for sample in dataset.samples)
    groups = len({sample.group for sample in dataset.samples})
    print(
        f"Dataset: {len(dataset.samples)} images "
        f"(sfw={sfw}, nsfw={nsfw}), {groups} groups, "
        f"{dataset.exact_duplicate_sets} exact duplicate sets"
    )


def _write_csv(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("Interrupted; completed per-model rows remain cached.", file=sys.stderr)
        raise SystemExit(130) from None
