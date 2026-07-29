from __future__ import annotations

from collections.abc import Iterable


def resolve_aliases(
    edges: Iterable[tuple[str, str]], known_targets: set[str]
) -> dict[str, str]:
    direct: dict[str, str] = {}
    for source, target in edges:
        if source == target:
            continue
        previous = direct.setdefault(source, target)
        if previous != target:
            raise ValueError(f"alias {source!r} has conflicting targets")

    resolved: dict[str, str] = {}
    for source in sorted(direct):
        path: list[str] = []
        current = source
        while current in direct:
            if current in path:
                cycle = " -> ".join([*path, current])
                raise ValueError(f"alias cycle detected: {cycle}")
            path.append(current)
            current = direct[current]
        if current not in known_targets:
            raise ValueError(f"alias {source!r} resolves to missing target {current!r}")
        resolved[source] = current
    return resolved
