# Search evaluation data

`queries.jsonl` uses graded relevance judgments from 0 to 3. The release set
must remain approximately 60% Chinese or mixed-language and 40% English, with
canonical, alias, translation, natural-language, character/copyright, artist,
and sensitive-tag slices.

The actual holdout judgments and BGE-M3 run are release data and must be
reviewed before `cargo xtask lexicon benchmark` can pass. The checked-in sample
only verifies the evaluator format.

Each judgment row contains `id`, `query`, `locale`, `slice`, optional
`entity_kind`, and a canonical-tag-to-relevance map. Candidate and baseline
runs contain `id` plus an ordered `results` array of canonical tags.

Run the release gate with an optimized `xtask`; it validates overall and slice
quality, exact/alias recall, the 300 MiB bundle budget, lexical timings, and
first/warmed semantic timings against the bundled shared ONNX Runtime:

```powershell
cargo run --release -p xtask -- lexicon benchmark `
  --queries tools/lexicon-pipeline/eval/queries.jsonl `
  --candidate-run tools/lexicon-pipeline/eval/e5-run.jsonl `
  --baseline-run tools/lexicon-pipeline/eval/bge-m3-run.jsonl
```
