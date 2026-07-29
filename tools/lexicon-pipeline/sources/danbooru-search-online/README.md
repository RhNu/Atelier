# DanbooruSearchOnline data snapshot

This directory contains a byte-for-byte snapshot of five data files from
[`SuzumiyaAkizuki/DanbooruSearchOnline`](https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline).
Atelier uses them as pinned enhancement inputs for the offline lexicon
pipeline. They are source data, not runtime application resources.

## Pinned upstream

- Repository: `SuzumiyaAkizuki/DanbooruSearchOnline`
- Commit: `73dc4395bd4152b1c54450a409cec341331c9a33`
- Commit date: `2026-07-29T04:57:34Z`
- Upstream paths: `origin_database/<file>`
- License stated by the repository root: GPL-3.0; the verbatim upstream
  license is retained in `LICENSE`

The upstream README contains separate Hugging Face front matter saying
`license: mit`, while the repository root `LICENSE` and GitHub repository
metadata identify GPL-3.0. Atelier follows the formal repository license file
for this snapshot and records the inconsistency instead of silently resolving
it.

The upstream project says its database was assembled from the Danbooru API,
LLM-generated semantic expansion and Chinese translation, with Bangumi API
lookups for Character and Copyright names. The snapshot does not contain
per-record provenance sufficient for Atelier to independently verify every
derived field. Retaining the GPL file does not override any rights or terms
that may apply to underlying Danbooru or Bangumi records.

## Files

- `tags_enhanced.csv`: selected tag vocabulary, translations, Wiki-derived
  text and sensitivity metadata
- `tag_aliases.parquet`: alias-to-canonical relationships
- `tag_groups.json`: tag group membership and localized group names
- `cooccurrence_clean.parquet`: tag-to-tag co-occurrence scores
- `tag_artist_cooc.parquet`: tag-to-Artist co-occurrence scores

Exact upstream blob IDs, sizes and SHA-256 checksums are in `SOURCE.json`.
`SHA256SUMS` provides a format suitable for common checksum tools.

The dataset files are stored with Git LFS. A pointer file is not a usable
pipeline input; run `git lfs pull` before normalization.

To update this snapshot, copy all five files from one explicit upstream commit,
replace `LICENSE` if it changed, update `SOURCE.json` and `SHA256SUMS`, then
rebuild and validate the Atelier bundle. Do not combine files from different
upstream commits.
