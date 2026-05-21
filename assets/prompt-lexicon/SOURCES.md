# Prompt Lexicon Sources

## Snapshot

- Snapshot date: 2026-05-20
- Local reference: `D:\Source\_Rust\nait\assets\prompt-lexicon\sources`
- Local reference commit: `4d5fe9abab48f942033288c439625ed3b2360dc5`
- Local reference working tree note: unrelated files were dirty; prompt lexicon
  source assets and generated lexicon last changed at commit
  `4d5fe9abab48f942033288c439625ed3b2360dc5`.
- Generated asset: `assets/prompt-lexicon/generated/lexicon.json`
- Builder: `cargo xtask lexicon build`

The source files in this directory were copied from the read-only `nait`
reference workspace, then regenerated through this repository's Rust `xtask`
builder. The checked-in generated schema is Atelier's own
`atelier-prompt-lexicon` v1 format.

## Checked-In Source Files

`translation-sources.json` names these source ids and checked-in files:

| Source id | File | Parser | Origin noted by `nait` |
|---|---|---|---|
| `aaalice_danbooru` | `translations/aaalice/danbooru.csv` | `simple_csv` | `Aaalice_NAI_Launcher` prompt Danbooru translations |
| `aaalice_danbooru_zh` | `translations/aaalice/danbooru_zh.csv` | `simple_csv` | `Aaalice_NAI_Launcher` prompt Danbooru translations |
| `aaalice_wai_characters` | `translations/aaalice/wai_characters.csv` | `reversed_csv` | `Aaalice_NAI_Launcher` prompt character translations |
| `aaalice_github_chening233` | `translations/aaalice/github_chening233.csv` | `github_csv` | `Aaalice_NAI_Launcher` prompt Danbooru translations |
| `aaalice_hf_danbooru_tags` | `translations/aaalice/hf_danbooru_tags.csv` | `alias_csv` | `Aaalice_NAI_Launcher` prompt Danbooru alias translations |
| `repo_weighted_danbooru` | `danbooru.csv` | `weighted_csv` | `nai-codex` style weighted Danbooru source |

`category-order.json` is derived from
`D:\Source\_Rust\nait\assets\prompt-lexicon\generated\lexicon.json` at the
same local reference commit. It records the reference Lexicon category and
subcategory browse order without copying UI code.

`translations/aaalice/github_chening233.csv` is trimmed in this repository to
`tag` and `danbooru_translation`. The copied reference CSV also contained
`danbooru_text` and `danbooru_url`; the Rust builder never reads those columns,
so they are not checked in here.

## Upstream Notes From `nait`

`nait` records two relevant upstream references:

- `Aaalice_NAI_Launcher` (`https://github.com/Aaalice233/Aaalice_NAI_Launcher`, MIT): Prompt Danbooru translation CSV resource mapping.
- `nai-codex` (`https://github.com/RhNu/nai-codex`, MIT): prompt lexicon generation flow, autocomplete contract, and Lexicon browsing/assembly interaction mapping.

`nait` does not record upstream source-asset commit SHAs for the copied CSV/JSON
inputs. Atelier therefore records the exact local `nait` commit snapshot
used for this import and keeps the upstream repository/license references from
`nait`'s own `CREDITS.md`.

Atelier copies static source assets from the local `nait` reference and
reimplements the builder/query code in this repository. It does not copy
Flutter, Vue, Tauri command, or application-service implementation from the
reference projects.
