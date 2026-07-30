# Atelier NSFW benchmark

This offline tool compares local and revision-pinned ONNX safety models against a two-folder
dataset:

```text
dataset/
  sfw/
  nsfw/
```

It evaluates:

- `deepghs/anime_dbrating` MobileNetV3 ratings.
- `deepghs/anime_rating` MobileNetV3 ratings.
- Waifu Diffusion Tagger v3 ratings.
- Mean/max score ensembles and group-cross-validated logistic fusion combinations.
- Uncertainty-band cascades that invoke a secondary model only for borderline negatives.

Model weights are downloaded to the normal Hugging Face cache and are not copied into the
repository. Revisions are pinned in `benchmark.toml`. Raw inference is cached per model and image
hash, so an interrupted run can resume.

Downloading a model for evaluation does not grant permission to redistribute it with Atelier.
Review each upstream model card and license before packaging model weights in a release.

## Run

From this directory:

```powershell
uv sync --locked
uv run --locked atelier-nsfw-benchmark
```

Defaults:

- Dataset: `../../temp/res-nsfw-benchmark`
- Output: `../../temp/nsfw-benchmark-results`

Override them or skip the large WD Tagger model:

```powershell
uv run --locked atelier-nsfw-benchmark `
  --dataset D:\images\nsfw-benchmark `
  --output D:\benchmarks\atelier-nsfw `
  --skip-model wd_tagger
```

Useful outputs:

- `REPORT.md`: dataset summary, ranking, selected configurations, and error paths.
- `recommendation.json`: fitted deployment threshold and optional logistic coefficients.
- `metrics.csv`: all configuration metrics.
- `folds.csv`: per-fold thresholds and metrics.
- `predictions.csv`: out-of-fold decision for every image/configuration.
- `scores.csv`: reusable raw model scores.
- `models.json`: exact model revision, SHA-256, size, outputs, and preprocessing.

After exporting scores with the Rust production adapter, feed them back into the benchmark:

```powershell
uv run --locked atelier-nsfw-production-compare `
  --rust ../../temp/nsfw-benchmark-results/rust-production-scores.csv `
  --python ../../temp/nsfw-benchmark-results/scores.csv `
  --output ../../temp/nsfw-benchmark-results/RUST_PRODUCTION_PARITY
```

The benchmark groups files by the leading `YYYY-MM-DD` filename segment. A generation date/batch
never appears in both train and validation for the same fold.

## Interpretation

Folder names are treated as ground truth. This corpus is useful for selecting candidates and
finding obvious regressions, but it is not a final independent holdout: model choice, fusion
choice, and final threshold are all informed by the same corpus. Before shipping a production
policy, freeze the selected configuration and validate it on separately labeled future images.

## Quality checks

```powershell
uv run --locked pytest
uv run --locked ruff format --check .
uv run --locked ruff check .
uv lock --check
```
