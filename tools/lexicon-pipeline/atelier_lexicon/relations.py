from __future__ import annotations

import math


def normalized_pmi(
    pair_count: int, left_count: int, right_count: int, total_count: int
) -> float:
    if min(pair_count, left_count, right_count, total_count) <= 0:
        return 0.0
    pair_probability = pair_count / total_count
    denominator = (left_count / total_count) * (right_count / total_count)
    pmi = math.log(pair_probability / denominator)
    return max(-1.0, min(1.0, pmi / -math.log(pair_probability)))
