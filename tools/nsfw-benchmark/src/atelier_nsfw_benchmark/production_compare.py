from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path

RATINGS = ("general", "sensitive", "questionable", "explicit")
POLICY_PATH = (
    Path(__file__).resolve().parents[4]
    / "crates"
    / "features"
    / "safety"
    / "policies"
    / "anime-rating-cascade-v1.json"
)
POLICY = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
PRIMARY_INTERCEPT = float(POLICY["primary_intercept"])
PRIMARY_COEFFICIENTS = tuple(float(value) for value in POLICY["primary_coefficients"])
PRIMARY_THRESHOLD = float(POLICY["primary_threshold"])
REVIEW_LOWER_BOUND = PRIMARY_THRESHOLD - float(POLICY["review_margin"])
REVIEW_INTERCEPT = float(POLICY["review_intercept"])
REVIEW_COEFFICIENTS = tuple(float(value) for value in POLICY["review_coefficients"])
REVIEW_THRESHOLD = float(POLICY["review_threshold"])


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare Rust production image-analysis scores with the Python benchmark."
    )
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = compare(args.rust, args.python)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.with_suffix(".json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    args.output.with_suffix(".md").write_text(render_markdown(report), encoding="utf-8")
    print(json.dumps(report, indent=2))


def compare(rust_path: Path, python_path: Path) -> dict[str, object]:
    rust_rows = _read_by_path(rust_path, "path")
    python_rows = _read_by_path(python_path, "relative_path")
    if rust_rows.keys() != python_rows.keys():
        missing_rust = sorted(python_rows.keys() - rust_rows.keys())
        missing_python = sorted(rust_rows.keys() - python_rows.keys())
        raise ValueError(
            "score paths differ; "
            f"missing Rust={missing_rust[:3]}, missing Python={missing_python[:3]}"
        )

    max_differences = {rating: 0.0 for rating in RATINGS}
    max_difference_paths = {rating: "" for rating in RATINGS}
    max_review_differences = {rating: 0.0 for rating in RATINGS}
    max_review_difference_paths = {rating: "" for rating in RATINGS}
    policy_disagreements: list[str] = []
    rust_confusion = {"tn": 0, "fp": 0, "fn": 0, "tp": 0}
    python_confusion = {"tn": 0, "fp": 0, "fn": 0, "tp": 0}
    rust_review_calls = 0
    python_review_calls = 0

    for path in sorted(rust_rows):
        rust = rust_rows[path]
        python = python_rows[path]
        label = 1 if rust["label"] == "nsfw" else 0
        for rating in RATINGS:
            difference = abs(
                float(rust[f"dbrating_{rating}"]) - float(python[f"dbrating_{rating}"])
            )
            if difference > max_differences[rating]:
                max_differences[rating] = difference
                max_difference_paths[rating] = path
            rust_review_value = rust.get(f"wd_{rating}", "")
            if rust_review_value:
                review_difference = abs(
                    float(rust_review_value) - float(python[f"wd_{rating}"])
                )
                if review_difference > max_review_differences[rating]:
                    max_review_differences[rating] = review_difference
                    max_review_difference_paths[rating] = path

        rust_decision = rust["final_sensitive"].lower() == "true"
        rust_review_calls += rust["review_called"].lower() == "true"
        python_decision, python_review = policy_decision(python)
        python_review_calls += python_review
        _record_confusion(rust_confusion, label, rust_decision)
        _record_confusion(python_confusion, label, python_decision)
        if rust_decision != python_decision:
            policy_disagreements.append(path)

    count = len(rust_rows)
    return {
        "images": count,
        "rust_confusion": rust_confusion,
        "python_confusion": python_confusion,
        "policy_disagreements": policy_disagreements,
        "max_primary_rating_absolute_difference": max_differences,
        "max_difference_paths": max_difference_paths,
        "max_review_rating_absolute_difference": max_review_differences,
        "max_review_difference_paths": max_review_difference_paths,
        "rust_review_calls": rust_review_calls,
        "rust_review_rate": rust_review_calls / count,
        "python_review_calls": python_review_calls,
        "python_review_rate": python_review_calls / count,
        "policy_version_retained": not policy_disagreements,
        "policy_id": POLICY["id"],
        "policy_version": POLICY["version"],
    }


def policy_decision(row: dict[str, str]) -> tuple[bool, bool]:
    primary = _logistic(
        tuple(float(row[f"dbrating_{rating}"]) for rating in RATINGS),
        PRIMARY_INTERCEPT,
        PRIMARY_COEFFICIENTS,
    )
    if primary >= PRIMARY_THRESHOLD:
        return True, False
    if primary < REVIEW_LOWER_BOUND:
        return False, False
    review = _logistic(
        tuple(float(row[f"wd_{rating}"]) for rating in RATINGS),
        REVIEW_INTERCEPT,
        REVIEW_COEFFICIENTS,
    )
    return review >= REVIEW_THRESHOLD, True


def render_markdown(report: dict[str, object]) -> str:
    rust = report["rust_confusion"]
    python = report["python_confusion"]
    differences = report["max_primary_rating_absolute_difference"]
    paths = report["max_difference_paths"]
    review_differences = report["max_review_rating_absolute_difference"]
    review_paths = report["max_review_difference_paths"]
    assert isinstance(rust, dict)
    assert isinstance(python, dict)
    assert isinstance(differences, dict)
    assert isinstance(paths, dict)
    assert isinstance(review_differences, dict)
    assert isinstance(review_paths, dict)
    lines = [
        "# Rust production score parity",
        "",
        f"- Images: {report['images']}",
        f"- Policy: {report['policy_id']} v{report['policy_version']}",
        (f"- Rust policy: TN {rust['tn']}, FP {rust['fp']}, FN {rust['fn']}, TP {rust['tp']}"),
        (
            "- Python policy: "
            f"TN {python['tn']}, FP {python['fp']}, FN {python['fn']}, TP {python['tp']}"
        ),
        f"- Policy disagreements: {len(report['policy_disagreements'])}",
        (
            f"- Rust WD calls: {report['rust_review_calls']} "
            f"({float(report['rust_review_rate']):.4%})"
        ),
        (
            f"- Python WD calls: {report['python_review_calls']} "
            f"({float(report['python_review_rate']):.4%})"
        ),
        f"- Policy version retained: {report['policy_version_retained']}",
        "",
        "## Maximum primary rating differences",
        "",
        "| Rating | Absolute difference | Image |",
        "|---|---:|---|",
    ]
    lines.extend(
        f"| {rating} | {float(differences[rating]):.9f} | `{paths[rating]}` |" for rating in RATINGS
    )
    lines.extend(
        [
            "",
            "## Maximum review rating differences",
            "",
            "| Rating | Absolute difference | Image |",
            "|---|---:|---|",
        ]
    )
    lines.extend(
        f"| {rating} | {float(review_differences[rating]):.9f} | `{review_paths[rating]}` |"
        for rating in RATINGS
    )
    return "\n".join(lines) + "\n"


def _read_by_path(path: Path, key: str) -> dict[str, dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as file:
        return {row[key]: row for row in csv.DictReader(file)}


def _logistic(
    values: tuple[float, ...], intercept: float, coefficients: tuple[float, ...]
) -> float:
    logit = intercept + sum(
        value * coefficient for value, coefficient in zip(values, coefficients, strict=True)
    )
    return 1.0 / (1.0 + math.exp(-logit))


def _record_confusion(confusion: dict[str, int], label: int, decision: bool) -> None:
    key = ("t" if decision == bool(label) else "f") + ("p" if decision else "n")
    confusion[key] += 1
