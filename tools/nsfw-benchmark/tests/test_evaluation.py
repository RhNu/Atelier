import numpy as np

from atelier_nsfw_benchmark.evaluation import choose_threshold


def test_threshold_prioritizes_target_recall_then_specificity() -> None:
    labels = np.asarray([0, 0, 0, 1, 1, 1])
    scores = np.asarray([0.1, 0.2, 0.8, 0.6, 0.7, 0.9])

    threshold = choose_threshold(labels, scores, recall_target=1.0)

    assert threshold == 0.6


def test_threshold_prefers_more_recall_when_specificity_is_tied() -> None:
    labels = np.asarray([0, 0, 1, 1])
    scores = np.asarray([0.1, 0.2, 0.6, 0.7])

    threshold = choose_threshold(labels, scores, recall_target=0.5)

    assert threshold == 0.6
