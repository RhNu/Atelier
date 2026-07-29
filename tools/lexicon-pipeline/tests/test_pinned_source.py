import hashlib
import json
from pathlib import Path


SOURCE_DIR = (
    Path(__file__).parents[1] / "sources" / "danbooru-search-online"
)


def test_pinned_dso_files_match_source_manifest() -> None:
    metadata = json.loads((SOURCE_DIR / "SOURCE.json").read_text(encoding="utf-8"))
    records = [*metadata["files"], metadata["license"]]

    for record in records:
        path = SOURCE_DIR / record["name"]
        assert path.is_file(), f"missing pinned DSO source: {path}"
        content = path.read_bytes()
        assert not content.startswith(b"version https://git-lfs.github.com/spec/"), (
            f"{path} is an unresolved Git LFS pointer; run `git lfs pull`"
        )
        assert len(content) == record["bytes"], f"size mismatch for {path}"
        assert hashlib.sha256(content).hexdigest() == record["sha256"], (
            f"SHA-256 mismatch for {path}"
        )
