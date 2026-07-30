from __future__ import annotations

import itertools
from dataclasses import dataclass

import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import (
    average_precision_score,
    balanced_accuracy_score,
    confusion_matrix,
    f1_score,
    fbeta_score,
    matthews_corrcoef,
    precision_score,
    recall_score,
    roc_auc_score,
)
from sklearn.model_selection import StratifiedGroupKFold


@dataclass
class EvaluationResult:
    name: str
    kind: str
    models: tuple[str, ...]
    feature_names: tuple[str, ...]
    metrics: dict[str, float | int | str]
    fold_rows: list[dict[str, float | int | str]]
    predictions: np.ndarray
    decisions: np.ndarray
    thresholds: np.ndarray
    final_threshold: float
    final_coefficients: list[float] | None
    final_intercept: float | None
    final_rule: dict[str, object] | None = None


def evaluate_all(
    rows: list[dict[str, str]],
    recall_target: float,
    folds: int,
    random_seed: int,
    model_sizes: dict[str, int],
) -> list[EvaluationResult]:
    labels = np.asarray([int(row["label"]) for row in rows], dtype=np.int64)
    groups = np.asarray([row["group"] for row in rows])
    numeric = {
        key: np.asarray([float(row[key]) for row in rows], dtype=np.float64)
        for key in rows[0]
        if key not in {"relative_path", "sha256", "label", "label_name", "group", "width", "height"}
        and not key.endswith("_latency_ms")
        and not key.startswith("scorer_")
    }
    latency = {
        key.removesuffix("_latency_ms"): np.asarray(
            [float(row[key]) for row in rows], dtype=np.float64
        )
        for key in rows[0]
        if key.endswith("_latency_ms")
    }
    recipes, dependencies = _score_recipes(numeric)
    splitter = StratifiedGroupKFold(
        n_splits=folds,
        shuffle=True,
        random_state=random_seed,
    )
    splits = list(splitter.split(np.zeros((len(labels), 1)), labels, groups))

    results: list[EvaluationResult] = []
    for name, scores in recipes.items():
        results.append(
            _evaluate_score(
                name=name,
                scores=scores,
                labels=labels,
                groups=groups,
                splits=splits,
                recall_target=recall_target,
                models=dependencies[name],
                model_sizes=model_sizes,
                latency=latency,
            )
        )

    feature_groups = _fusion_feature_groups(numeric)
    fusion_sets = _fusion_sets(feature_groups)
    for fusion_name, group_names in fusion_sets.items():
        feature_names = tuple(feature for group in group_names for feature in feature_groups[group])
        matrix = np.column_stack([numeric[feature] for feature in feature_names])
        for c_value in (0.1, 1.0, 10.0):
            results.append(
                _evaluate_logistic(
                    name=f"fusion__{fusion_name}__c_{c_value:g}",
                    matrix=matrix,
                    labels=labels,
                    groups=groups,
                    splits=splits,
                    recall_target=recall_target,
                    models=tuple(group_names),
                    feature_names=feature_names,
                    c_value=c_value,
                    model_sizes=model_sizes,
                    latency=latency,
                    random_seed=random_seed,
                )
            )

    if "anime_dbrating" in feature_groups:
        for secondary in ("anime_rating", "wd_tagger"):
            if secondary not in feature_groups:
                continue
            for margin in (0.02, 0.05, 0.1, 0.2):
                results.append(
                    _evaluate_cascade(
                        name=f"cascade__anime_dbrating__{secondary}__margin_{margin:g}",
                        primary_matrix=np.column_stack(
                            [numeric[name] for name in feature_groups["anime_dbrating"]]
                        ),
                        secondary_matrix=np.column_stack(
                            [numeric[name] for name in feature_groups[secondary]]
                        ),
                        labels=labels,
                        groups=groups,
                        splits=splits,
                        recall_target=recall_target,
                        primary_features=feature_groups["anime_dbrating"],
                        secondary_features=feature_groups[secondary],
                        secondary_model=secondary,
                        margin=margin,
                        model_sizes=model_sizes,
                        latency=latency,
                        random_seed=random_seed,
                    )
                )
    return results


def choose_threshold(labels: np.ndarray, scores: np.ndarray, recall_target: float) -> float:
    order = np.argsort(-scores, kind="mergesort")
    sorted_scores = scores[order]
    sorted_labels = labels[order]
    cumulative_tp = np.cumsum(sorted_labels == 1)
    cumulative_fp = np.cumsum(sorted_labels == 0)
    group_ends = np.flatnonzero(np.concatenate([sorted_scores[1:] != sorted_scores[:-1], [True]]))
    positives = max(int(np.sum(labels == 1)), 1)
    negatives = max(int(np.sum(labels == 0)), 1)
    candidates = [
        (
            float(np.nextafter(sorted_scores[0], np.inf)),
            0,
            0,
        ),
        *[
            (
                float(sorted_scores[index]),
                int(cumulative_tp[index]),
                int(cumulative_fp[index]),
            )
            for index in group_ends
        ],
    ]
    best: tuple[float, float, float, float] | None = None
    best_threshold = 0.0
    for threshold, true_positives, false_positives in candidates:
        recall = true_positives / positives
        specificity = (negatives - false_positives) / negatives
        predicted_positive = true_positives + false_positives
        precision = true_positives / predicted_positive if predicted_positive else 0.0
        feasible = float(recall >= recall_target)
        if feasible:
            key = (feasible, specificity, recall, precision, float(threshold))
        else:
            key = (feasible, recall, specificity, precision, float(threshold))
        if best is None or key > best:
            best = key
            best_threshold = float(threshold)
    return best_threshold


def rank_results(
    results: list[EvaluationResult],
    compact_limit_mb: float,
    recall_target: float,
) -> dict[str, str]:
    selectable = [result for result in results if result.kind != "fixed"]

    def key(result: EvaluationResult) -> tuple[float, ...]:
        metrics = result.metrics
        recall = float(metrics["recall"])
        return (
            float(recall >= recall_target),
            float(metrics["f2"]),
            recall,
            float(metrics["specificity"]),
            float(metrics["average_precision"]),
            -float(metrics["model_size_mb"]),
            -float(metrics["mean_latency_ms"]),
        )

    overall = max(selectable, key=key)
    compact_candidates = [
        result
        for result in selectable
        if float(result.metrics["model_size_mb"]) <= compact_limit_mb
    ]
    compact = max(compact_candidates, key=key) if compact_candidates else overall
    return {"overall": overall.name, "compact": compact.name}


def _evaluate_score(
    name: str,
    scores: np.ndarray,
    labels: np.ndarray,
    groups: np.ndarray,
    splits: list[tuple[np.ndarray, np.ndarray]],
    recall_target: float,
    models: tuple[str, ...],
    model_sizes: dict[str, int],
    latency: dict[str, np.ndarray],
) -> EvaluationResult:
    oof = scores.copy()
    decisions = np.zeros(len(labels), dtype=bool)
    thresholds = np.zeros(len(labels), dtype=np.float64)
    fold_rows = []
    for fold, (train, test) in enumerate(splits):
        threshold = choose_threshold(labels[train], scores[train], recall_target)
        fold_decisions = scores[test] >= threshold
        decisions[test] = fold_decisions
        thresholds[test] = threshold
        fold_rows.append(
            {
                "config": name,
                "fold": fold,
                "threshold": threshold,
                **_classification_metrics(labels[test], scores[test], fold_decisions),
                "test_groups": ",".join(sorted(set(groups[test]))),
            }
        )
    final_threshold = choose_threshold(labels, scores, recall_target)
    metrics = _classification_metrics(labels, oof, decisions)
    _attach_costs(metrics, models, model_sizes, latency)
    return EvaluationResult(
        name=name,
        kind="score",
        models=models,
        feature_names=(name,),
        metrics=metrics,
        fold_rows=fold_rows,
        predictions=oof,
        decisions=decisions,
        thresholds=thresholds,
        final_threshold=final_threshold,
        final_coefficients=None,
        final_intercept=None,
    )


def _evaluate_logistic(
    name: str,
    matrix: np.ndarray,
    labels: np.ndarray,
    groups: np.ndarray,
    splits: list[tuple[np.ndarray, np.ndarray]],
    recall_target: float,
    models: tuple[str, ...],
    feature_names: tuple[str, ...],
    c_value: float,
    model_sizes: dict[str, int],
    latency: dict[str, np.ndarray],
    random_seed: int,
) -> EvaluationResult:
    oof = np.zeros(len(labels), dtype=np.float64)
    decisions = np.zeros(len(labels), dtype=bool)
    thresholds = np.zeros(len(labels), dtype=np.float64)
    fold_rows = []
    for fold, (train, test) in enumerate(splits):
        classifier = _logistic(c_value, random_seed)
        classifier.fit(matrix[train], labels[train])
        train_scores = classifier.predict_proba(matrix[train])[:, 1]
        test_scores = classifier.predict_proba(matrix[test])[:, 1]
        threshold = choose_threshold(labels[train], train_scores, recall_target)
        fold_decisions = test_scores >= threshold
        oof[test] = test_scores
        decisions[test] = fold_decisions
        thresholds[test] = threshold
        fold_rows.append(
            {
                "config": name,
                "fold": fold,
                "threshold": threshold,
                **_classification_metrics(labels[test], test_scores, fold_decisions),
                "test_groups": ",".join(sorted(set(groups[test]))),
            }
        )

    final_model = _logistic(c_value, random_seed)
    final_model.fit(matrix, labels)
    final_scores = final_model.predict_proba(matrix)[:, 1]
    final_threshold = choose_threshold(labels, final_scores, recall_target)
    metrics = _classification_metrics(labels, oof, decisions)
    _attach_costs(metrics, models, model_sizes, latency)
    return EvaluationResult(
        name=name,
        kind="logistic",
        models=models,
        feature_names=feature_names,
        metrics=metrics,
        fold_rows=fold_rows,
        predictions=oof,
        decisions=decisions,
        thresholds=thresholds,
        final_threshold=final_threshold,
        final_coefficients=final_model.coef_[0].tolist(),
        final_intercept=float(final_model.intercept_[0]),
    )


def _evaluate_cascade(
    name: str,
    primary_matrix: np.ndarray,
    secondary_matrix: np.ndarray,
    labels: np.ndarray,
    groups: np.ndarray,
    splits: list[tuple[np.ndarray, np.ndarray]],
    recall_target: float,
    primary_features: tuple[str, ...],
    secondary_features: tuple[str, ...],
    secondary_model: str,
    margin: float,
    model_sizes: dict[str, int],
    latency: dict[str, np.ndarray],
    random_seed: int,
) -> EvaluationResult:
    oof = np.zeros(len(labels), dtype=np.float64)
    decisions = np.zeros(len(labels), dtype=bool)
    thresholds = np.zeros(len(labels), dtype=np.float64)
    fallback_mask = np.zeros(len(labels), dtype=bool)
    fold_rows = []
    for fold, (train, test) in enumerate(splits):
        primary = _logistic(10.0, random_seed)
        secondary = _logistic(10.0, random_seed)
        primary.fit(primary_matrix[train], labels[train])
        secondary.fit(secondary_matrix[train], labels[train])
        primary_train = primary.predict_proba(primary_matrix[train])[:, 1]
        secondary_train = secondary.predict_proba(secondary_matrix[train])[:, 1]
        primary_threshold = choose_threshold(labels[train], primary_train, recall_target)
        secondary_threshold = choose_threshold(labels[train], secondary_train, recall_target)

        primary_test = primary.predict_proba(primary_matrix[test])[:, 1]
        secondary_test = secondary.predict_proba(secondary_matrix[test])[:, 1]
        primary_decision = primary_test >= primary_threshold
        fallback = (~primary_decision) & (primary_test >= primary_threshold - margin)
        fold_decisions = primary_decision | (fallback & (secondary_test >= secondary_threshold))
        composite_score = np.where(fallback, np.maximum(primary_test, secondary_test), primary_test)
        oof[test] = composite_score
        decisions[test] = fold_decisions
        thresholds[test] = primary_threshold
        fallback_mask[test] = fallback
        fold_rows.append(
            {
                "config": name,
                "fold": fold,
                "threshold": primary_threshold,
                "secondary_threshold": secondary_threshold,
                "fallback_rate": float(np.mean(fallback)),
                **_classification_metrics(labels[test], composite_score, fold_decisions),
                "test_groups": ",".join(sorted(set(groups[test]))),
            }
        )

    primary_final = _logistic(10.0, random_seed)
    secondary_final = _logistic(10.0, random_seed)
    primary_final.fit(primary_matrix, labels)
    secondary_final.fit(secondary_matrix, labels)
    primary_full = primary_final.predict_proba(primary_matrix)[:, 1]
    secondary_full = secondary_final.predict_proba(secondary_matrix)[:, 1]
    primary_threshold = choose_threshold(labels, primary_full, recall_target)
    secondary_threshold = choose_threshold(labels, secondary_full, recall_target)

    models = ("anime_dbrating", secondary_model)
    metrics = _classification_metrics(labels, oof, decisions)
    _attach_costs(metrics, models, model_sizes, latency)
    invocation_rate = float(np.mean(fallback_mask))
    primary_latency = latency.get("anime_dbrating", np.zeros(1))
    secondary_latency = latency.get(secondary_model, np.zeros(1))
    metrics["secondary_invocation_rate"] = invocation_rate
    metrics["mean_latency_ms"] = float(
        np.mean(primary_latency) + invocation_rate * np.mean(secondary_latency)
    )
    metrics["p95_latency_ms"] = float(
        np.percentile(primary_latency, 95) + np.percentile(secondary_latency, 95)
    )
    return EvaluationResult(
        name=name,
        kind="cascade",
        models=models,
        feature_names=(*primary_features, *secondary_features),
        metrics=metrics,
        fold_rows=fold_rows,
        predictions=oof,
        decisions=decisions,
        thresholds=thresholds,
        final_threshold=primary_threshold,
        final_coefficients=primary_final.coef_[0].tolist(),
        final_intercept=float(primary_final.intercept_[0]),
        final_rule={
            "type": "negative_uncertainty_fallback",
            "margin": margin,
            "primary": {
                "model": "anime_dbrating",
                "features": primary_features,
                "threshold": primary_threshold,
                "coefficients": primary_final.coef_[0].tolist(),
                "intercept": float(primary_final.intercept_[0]),
            },
            "secondary": {
                "model": secondary_model,
                "features": secondary_features,
                "threshold": secondary_threshold,
                "coefficients": secondary_final.coef_[0].tolist(),
                "intercept": float(secondary_final.intercept_[0]),
            },
        },
    )


def _logistic(c_value: float, random_seed: int) -> LogisticRegression:
    return LogisticRegression(
        C=c_value,
        class_weight="balanced",
        max_iter=1_000,
        solver="lbfgs",
        random_state=random_seed,
    )


def _classification_metrics(
    labels: np.ndarray,
    scores: np.ndarray,
    decisions: np.ndarray,
) -> dict[str, float | int]:
    tn, fp, fn, tp = confusion_matrix(labels, decisions, labels=[0, 1]).ravel()
    return {
        "roc_auc": roc_auc_score(labels, scores),
        "average_precision": average_precision_score(labels, scores),
        "recall": recall_score(labels, decisions, zero_division=0),
        "specificity": recall_score(labels, decisions, pos_label=0, zero_division=0),
        "precision": precision_score(labels, decisions, zero_division=0),
        "f1": f1_score(labels, decisions, zero_division=0),
        "f2": fbeta_score(labels, decisions, beta=2, zero_division=0),
        "balanced_accuracy": balanced_accuracy_score(labels, decisions),
        "mcc": matthews_corrcoef(labels, decisions),
        "tn": int(tn),
        "fp": int(fp),
        "fn": int(fn),
        "tp": int(tp),
    }


def _attach_costs(
    metrics: dict[str, float | int],
    models: tuple[str, ...],
    model_sizes: dict[str, int],
    latency: dict[str, np.ndarray],
) -> None:
    metrics["models"] = ",".join(models)
    metrics["model_size_mb"] = sum(model_sizes.get(model, 0) for model in models) / 1_000_000
    arrays = [latency[model] for model in models if model in latency]
    total_latency = np.sum(np.column_stack(arrays), axis=1) if arrays else np.zeros(1)
    metrics["mean_latency_ms"] = float(np.mean(total_latency))
    metrics["p95_latency_ms"] = float(np.percentile(total_latency, 95))


def _score_recipes(
    numeric: dict[str, np.ndarray],
) -> tuple[dict[str, np.ndarray], dict[str, tuple[str, ...]]]:
    recipes: dict[str, np.ndarray] = {}
    dependencies: dict[str, tuple[str, ...]] = {}

    def add(name: str, score: np.ndarray, models: tuple[str, ...]) -> None:
        recipes[name] = np.clip(score, 0.0, 1.0)
        dependencies[name] = models

    derived: dict[str, tuple[np.ndarray, tuple[str, ...]]] = {}
    if {"dbrating_questionable", "dbrating_explicit"} <= numeric.keys():
        derived["dbrating_qe"] = (
            numeric["dbrating_questionable"] + numeric["dbrating_explicit"],
            ("anime_dbrating",),
        )
        derived["dbrating_not_general"] = (
            1.0 - numeric["dbrating_general"],
            ("anime_dbrating",),
        )
        derived["dbrating_explicit"] = (
            numeric["dbrating_explicit"],
            ("anime_dbrating",),
        )
    if {"rating_safe", "rating_r15", "rating_r18"} <= numeric.keys():
        derived["anime_rating_r18"] = (numeric["rating_r18"], ("anime_rating",))
        derived["anime_rating_not_safe"] = (
            1.0 - numeric["rating_safe"],
            ("anime_rating",),
        )
    if {"wd_general", "wd_sensitive", "wd_questionable", "wd_explicit"} <= numeric.keys():
        derived["wd_qe"] = (
            numeric["wd_questionable"] + numeric["wd_explicit"],
            ("wd_tagger",),
        )
        derived["wd_not_general"] = (1.0 - numeric["wd_general"], ("wd_tagger",))
    for name, (score, models) in derived.items():
        add(name, score, models)

    canonical = {
        name: values
        for name, values in {
            "dbrating": recipes.get("dbrating_qe"),
            "rating": recipes.get("anime_rating_r18"),
            "wd": recipes.get("wd_qe"),
        }.items()
        if values is not None
    }
    for length in range(2, len(canonical) + 1):
        for names in itertools.combinations(canonical, length):
            arrays = [canonical[item] for item in names]
            models = tuple(
                {
                    "dbrating": "anime_dbrating",
                    "rating": "anime_rating",
                    "wd": "wd_tagger",
                }[item]
                for item in names
            )
            add(f"mean__{'__'.join(names)}", np.mean(arrays, axis=0), models)
            add(f"max__{'__'.join(names)}", np.max(arrays, axis=0), models)
    return recipes, dependencies


def _fusion_feature_groups(numeric: dict[str, np.ndarray]) -> dict[str, tuple[str, ...]]:
    candidates = {
        "anime_dbrating": (
            "dbrating_general",
            "dbrating_sensitive",
            "dbrating_questionable",
            "dbrating_explicit",
        ),
        "anime_rating": ("rating_safe", "rating_r15", "rating_r18"),
        "wd_tagger": ("wd_general", "wd_sensitive", "wd_questionable", "wd_explicit"),
    }
    return {
        name: features
        for name, features in candidates.items()
        if all(feature in numeric for feature in features)
    }


def _fusion_sets(
    groups: dict[str, tuple[str, ...]],
) -> dict[str, tuple[str, ...]]:
    available = tuple(groups)
    result: dict[str, tuple[str, ...]] = {}
    for length in range(1, len(available) + 1):
        for names in itertools.combinations(available, length):
            result["__".join(names)] = names
    return result
