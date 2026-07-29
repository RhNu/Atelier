from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any


@dataclass(frozen=True)
class LocalizedText:
    locale: str
    text: str


@dataclass
class Entity:
    id: int
    canonical_name: str
    primary_translation: str
    kind: str
    category: str
    post_count: int
    rating: str
    aliases: list[str] = field(default_factory=list)
    translations: list[LocalizedText] = field(default_factory=list)
    wiki: list[LocalizedText] = field(default_factory=list)
    groups: list[str] = field(default_factory=list)
    expansion_terms: list[str] = field(default_factory=list)

    def to_json(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class Relation:
    source_entity_id: int
    target_entity_id: int
    relation: str
    npmi: float


@dataclass(frozen=True)
class SourceRecord:
    id: str
    url: str
    snapshot: str
    sha256: str
    license: str
