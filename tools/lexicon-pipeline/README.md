# Atelier Lexicon Pipeline

This offline pipeline creates the normalized inputs consumed by
`cargo xtask lexicon bundle`. It does not run inside Atelier.

The checked-in implementation is original Atelier code. It is informed by the
data model and published algorithms of:

- <https://github.com/SuzumiyaAkizuki/danbooru-tag-pipeline>
- <https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline>

The DSO-derived enhancement inputs are pinned in
`sources/danbooru-search-online`. The snapshot directory contains its upstream
license, exact commit, file-level provenance, and SHA-256 checksums. Static
dataset files use Git LFS; run `git lfs pull` before the pipeline.

Do not place API keys in configuration files. The optional enrichment stage
reads `OPENAI_API_KEY`, `OPENAI_BASE_URL`, and `OPENAI_MODEL` from the
environment and stores content-addressed responses below the selected cache
directory.

```powershell
python -m venv .venv
.venv\Scripts\pip install -e "tools/lexicon-pipeline[dev,semantic]"
atelier-lexicon fetch-selected-tags --output upstream
atelier-lexicon normalize --input upstream --output build
atelier-lexicon enrich --input build/entities.jsonl `
  --output build/entities.enriched.jsonl --cache .cache/lexicon-llm `
  --batch-size 1000
atelier-lexicon semantic --entities build/entities.enriched.jsonl `
  --output build/semantic
cargo xtask lexicon bundle --input tools/lexicon-pipeline/build `
  --output apps/desktop/src-tauri/resources/lexicon
```

`normalize` expects:

- `tags.jsonl` from `fetch-selected-tags` (or the resumable full
  `fetch-tags` command); this is mandatory so runtime entity IDs remain real
  Danbooru tag IDs
- the checked-in DSO snapshot containing `tags_enhanced.csv` and the optional
  alias, group, tag co-occurrence, and Artist co-occurrence files

`fetch-selected-tags --names-from` and `normalize --dso-input` default to the
checked-in snapshot. They can be overridden explicitly when auditing a
replacement snapshot.

The main partition includes General, Copyright, and Character tags with at
least 100 posts. Artist tags are emitted as a separate entity kind. Other tags
are written to `cold/entities.parquet` and are not bundled by Atelier.

The enrichment stage reads `OPENAI_API_KEY`, `OPENAI_BASE_URL`, and
`OPENAI_MODEL` only from the environment. It uploads JSONL inputs with
`purpose=batch`, creates `/v1/chat/completions` batches, persists batch IDs
while polling, and downloads output/error files. Requests and successful
results are content-addressed, so interrupted batches resume and partial
failures only retry missing entities. The 1,000 request default splits the 51k
vocabulary into roughly 52 independently recoverable batches, limiting the
blast radius and diagnostic size of any provider-side failure.

Enrichment preserves existing translations, Wiki text, and expansion terms.
`cargo xtask lexicon bundle` automatically prefers
`entities.enriched.jsonl`, requires matching batch provenance, and rejects
semantic vectors built from a different entity file. The semantic stage uses
checkpointed memory maps and a revision-pinned `multilingual-e5-small` qint8
model.
