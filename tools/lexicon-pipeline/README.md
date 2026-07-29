# Atelier Lexicon Pipeline

This offline pipeline creates the normalized inputs consumed by
`cargo xtask lexicon bundle`. It does not run inside Atelier.

The checked-in implementation is original Atelier code. It is informed by the
data model and published algorithms of:

- <https://github.com/SuzumiyaAkizuki/danbooru-tag-pipeline>
- <https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline>

Do not place API keys in configuration files. The optional enrichment stage
reads `OPENAI_API_KEY`, `OPENAI_BASE_URL`, and `OPENAI_MODEL` from the
environment and stores content-addressed responses below the selected cache
directory.

```powershell
python -m venv .venv
.venv\Scripts\pip install -e "tools/lexicon-pipeline[dev,semantic]"
atelier-lexicon fetch-tags --output upstream
atelier-lexicon normalize --input upstream --output build
atelier-lexicon enrich --input build/entities.jsonl `
  --output build/entities.enriched.jsonl --cache .cache/lexicon-llm
atelier-lexicon semantic --entities build/entities.enriched.jsonl `
  --output build/semantic
cargo xtask lexicon bundle --input tools/lexicon-pipeline/build `
  --output apps/desktop/src-tauri/resources/lexicon
```

`normalize` expects:

- `tags.jsonl` from the resumable `fetch-tags` command; this is mandatory so
  runtime entity IDs remain real Danbooru tag IDs
- `tags_enhanced.csv`
- optional `tag_aliases.parquet`
- optional `tag_groups.json`
- optional `cooccurrence_clean.parquet`
- optional `tag_artist_cooc.parquet`

The main partition includes General, Copyright, and Character tags with at
least 100 posts. Artist tags are emitted as a separate entity kind. Other tags
are written to `cold/entities.parquet` and are not bundled by Atelier.

The enrichment stage reads `OPENAI_API_KEY`, `OPENAI_BASE_URL`, and
`OPENAI_MODEL` only from the environment. Its requests use strict JSON Schema,
bounded retries, and content-addressed response files, so a failed run can be
resumed without repeating completed calls. The semantic stage similarly uses
checkpointed memory maps and a revision-pinned `multilingual-e5-small` qint8
model.
