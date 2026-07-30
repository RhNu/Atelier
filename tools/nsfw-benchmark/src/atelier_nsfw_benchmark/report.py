from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any

from .dataset import DatasetSummary
from .evaluation import EvaluationResult


def write_reports(
    output_dir: Path,
    dataset: DatasetSummary,
    rows: list[dict[str, str]],
    results: list[EvaluationResult],
    selected: dict[str, str],
    model_info: list[dict[str, Any]],
    config_snapshot: dict[str, Any],
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    by_name = {result.name: result for result in results}
    ordered = sorted(
        results,
        key=lambda result: (
            -float(result.metrics["recall"]),
            -float(result.metrics["specificity"]),
            -float(result.metrics["average_precision"]),
        ),
    )

    metric_rows = [
        {
            "config": result.name,
            "kind": result.kind,
            "final_threshold": result.final_threshold,
            **result.metrics,
        }
        for result in ordered
    ]
    _write_csv(output_dir / "metrics.csv", metric_rows)
    _write_csv(
        output_dir / "folds.csv",
        [row for result in results for row in result.fold_rows],
    )
    prediction_rows = []
    for result in results:
        for row, score, decision, threshold in zip(
            rows,
            result.predictions,
            result.decisions,
            result.thresholds,
            strict=True,
        ):
            prediction_rows.append(
                {
                    "config": result.name,
                    "relative_path": row["relative_path"],
                    "label": row["label"],
                    "group": row["group"],
                    "score": score,
                    "threshold": threshold,
                    "prediction": int(decision),
                    "correct": int(int(row["label"]) == int(decision)),
                }
            )
    _write_csv(output_dir / "predictions.csv", prediction_rows)
    (output_dir / "models.json").write_text(
        json.dumps(model_info, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    recommendation = {
        "selection": selected,
        "configs": {slot: _deployment_record(by_name[name]) for slot, name in selected.items()},
        "benchmark": config_snapshot,
        "warning": (
            "Thresholds and fusion coefficients were fitted on this local dataset. "
            "Validate on a separately labeled holdout before shipping."
        ),
    }
    (output_dir / "recommendation.json").write_text(
        json.dumps(recommendation, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    (output_dir / "REPORT.md").write_text(
        _markdown_report(dataset, rows, ordered, selected, by_name),
        encoding="utf-8",
    )


def _deployment_record(result: EvaluationResult) -> dict[str, Any]:
    return {
        "config": result.name,
        "kind": result.kind,
        "models": result.models,
        "feature_names": result.feature_names,
        "threshold": result.final_threshold,
        "coefficients": result.final_coefficients,
        "intercept": result.final_intercept,
        "rule": result.final_rule,
        "cross_validated_metrics": result.metrics,
    }


def _markdown_report(
    dataset: DatasetSummary,
    rows: list[dict[str, str]],
    ordered: list[EvaluationResult],
    selected: dict[str, str],
    by_name: dict[str, EvaluationResult],
) -> str:
    sfw = sum(sample.label == 0 for sample in dataset.samples)
    nsfw = sum(sample.label == 1 for sample in dataset.samples)
    groups = len({sample.group for sample in dataset.samples})
    lines = [
        "# Atelier NSFW benchmark",
        "",
        "## Dataset",
        "",
        f"- Images: {len(dataset.samples)} (`sfw={sfw}`, `nsfw={nsfw}`)",
        f"- Date/name groups: {groups}",
        f"- Exact duplicate sets: {dataset.exact_duplicate_sets}",
        "- Validation: stratified group folds; a filename date group never crosses train/test.",
        "",
        "## Selected configurations",
        "",
    ]
    for slot, name in selected.items():
        result = by_name[name]
        metrics = result.metrics
        lines.extend(
            [
                f"### {slot}: `{name}`",
                "",
                (
                    f"Recall {float(metrics['recall']):.3f}, "
                    f"specificity {float(metrics['specificity']):.3f}, "
                    f"precision {float(metrics['precision']):.3f}, "
                    f"AP {float(metrics['average_precision']):.3f}, "
                    f"FN {metrics['fn']}, FP {metrics['fp']}, "
                    f"models {float(metrics['model_size_mb']):.1f} MB, "
                    f"mean latency {float(metrics['mean_latency_ms']):.1f} ms/image."
                ),
                "",
                (
                    "The deployment threshold/coefficients are fitted on all benchmark images; "
                    "the metrics above use out-of-fold predictions and fold-local thresholds."
                ),
                "",
            ]
        )

    lines.extend(
        [
            "## Ranking",
            "",
            "| Config | Recall | Specificity | Precision | AP | FN | FP | MB | ms/image |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for result in ordered[:20]:
        m = result.metrics
        lines.append(
            f"| `{result.name}` | {float(m['recall']):.3f} | "
            f"{float(m['specificity']):.3f} | {float(m['precision']):.3f} | "
            f"{float(m['average_precision']):.3f} | {m['fn']} | {m['fp']} | "
            f"{float(m['model_size_mb']):.1f} | {float(m['mean_latency_ms']):.1f} |"
        )

    for slot, name in selected.items():
        result = by_name[name]
        false_negatives = [
            rows[index]["relative_path"]
            for index, (label, decision) in enumerate(
                zip(
                    (int(row["label"]) for row in rows),
                    result.decisions,
                    strict=True,
                )
            )
            if label == 1 and not decision
        ]
        false_positives = [
            rows[index]["relative_path"]
            for index, (label, decision) in enumerate(
                zip(
                    (int(row["label"]) for row in rows),
                    result.decisions,
                    strict=True,
                )
            )
            if label == 0 and decision
        ]
        lines.extend(["", f"## Errors: {slot}", "", "### False negatives", ""])
        lines.extend(f"- `{path}`" for path in false_negatives[:30])
        if not false_negatives:
            lines.append("- None")
        lines.extend(["", "### False positives", ""])
        lines.extend(f"- `{path}`" for path in false_positives[:30])
        if not false_positives:
            lines.append("- None")

    lines.extend(
        [
            "",
            "## Interpretation limits",
            "",
            "- Folder labels are treated as ground truth but contain only two classes.",
            (
                "- Same-day generation batches are grouped, but this is not an "
                "independent future holdout."
            ),
            "- Model selection, threshold selection, and reporting use the same corpus.",
            "- Review error lists before changing Atelier's production policy.",
            "",
        ]
    )
    return "\n".join(lines)


def _write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    fieldnames = list(rows[0])
    with path.open("w", encoding="utf-8", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
