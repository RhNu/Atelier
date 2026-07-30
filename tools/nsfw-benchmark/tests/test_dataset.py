from pathlib import Path

from PIL import Image

from atelier_nsfw_benchmark.dataset import discover_dataset


def test_discovers_labels_and_groups(tmp_path: Path) -> None:
    for label in ("sfw", "nsfw"):
        folder = tmp_path / label
        folder.mkdir()
        color = "white" if label == "sfw" else "black"
        Image.new("RGB", (32, 48), color).save(folder / f"2026-07-29-{label}.png")

    summary = discover_dataset(tmp_path)

    assert len(summary.samples) == 2
    assert {sample.label for sample in summary.samples} == {0, 1}
    assert {sample.group for sample in summary.samples} == {"2026-07-29"}
