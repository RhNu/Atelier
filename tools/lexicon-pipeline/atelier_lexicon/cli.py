from __future__ import annotations

import argparse
from pathlib import Path

from .enrich import DEFAULT_BATCH_SIZE, enrich_file
from .fetch import fetch_danbooru_tags, fetch_selected_danbooru_tags
from .normalize import normalize
from .semantic import build_semantic_assets

DEFAULT_DSO_SOURCE = (
    Path(__file__).parents[1] / "sources" / "danbooru-search-online"
)


def main() -> None:
    parser = argparse.ArgumentParser(description="Atelier Danbooru lexicon pipeline")
    subcommands = parser.add_subparsers(dest="command", required=True)
    fetch_command = subcommands.add_parser("fetch-tags")
    fetch_command.add_argument("--output", type=Path, required=True)
    fetch_command.add_argument("--max-pages", type=int)
    fetch_selected_command = subcommands.add_parser("fetch-selected-tags")
    fetch_selected_command.add_argument(
        "--names-from",
        type=Path,
        default=DEFAULT_DSO_SOURCE / "tags_enhanced.csv",
    )
    fetch_selected_command.add_argument("--output", type=Path, required=True)
    fetch_selected_command.add_argument(
        "--deny",
        type=Path,
        default=Path(__file__).parents[1] / "curation" / "deny.json",
    )
    enrich_command = subcommands.add_parser("enrich")
    enrich_command.add_argument("--input", type=Path, required=True)
    enrich_command.add_argument("--output", type=Path, required=True)
    enrich_command.add_argument("--cache", type=Path, required=True)
    enrich_command.add_argument(
        "--batch-size", type=int, default=DEFAULT_BATCH_SIZE
    )
    enrich_command.add_argument("--poll-seconds", type=float, default=30)
    enrich_command.add_argument("--max-wait-seconds", type=float, default=90_000)
    semantic_command = subcommands.add_parser("semantic")
    semantic_command.add_argument("--entities", type=Path, required=True)
    semantic_command.add_argument("--output", type=Path, required=True)
    semantic_command.add_argument("--batch-size", type=int, default=64)
    normalize_command = subcommands.add_parser("normalize")
    normalize_command.add_argument("--input", type=Path, required=True)
    normalize_command.add_argument("--output", type=Path, required=True)
    normalize_command.add_argument(
        "--dso-input",
        type=Path,
        default=DEFAULT_DSO_SOURCE,
        help="pinned DanbooruSearchOnline data snapshot",
    )
    normalize_command.add_argument(
        "--allow",
        type=Path,
        default=Path(__file__).parents[1] / "curation" / "allow.json",
    )
    normalize_command.add_argument(
        "--deny",
        type=Path,
        default=Path(__file__).parents[1] / "curation" / "deny.json",
    )
    args = parser.parse_args()
    if args.command == "fetch-tags":
        fetch_danbooru_tags(args.output, args.max_pages)
    elif args.command == "fetch-selected-tags":
        fetch_selected_danbooru_tags(args.names_from, args.output, args.deny)
    elif args.command == "enrich":
        enrich_file(
            args.input,
            args.output,
            args.cache,
            args.batch_size,
            args.poll_seconds,
            args.max_wait_seconds,
        )
    elif args.command == "semantic":
        build_semantic_assets(args.entities, args.output, args.batch_size)
    elif args.command == "normalize":
        normalize(
            args.input,
            args.output,
            args.allow,
            args.deny,
            args.dso_input,
        )


if __name__ == "__main__":
    main()
