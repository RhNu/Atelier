# Agent tooling implementation

The Agent edits text and generation settings. Optional user-enabled vision reads generation
outputs; image guidance, Vibe, and pixel editing remain outside its mutation surface.

## Commit stages

1. Editing consistency: host-owned observations, atomic draft operations, exact text replacement,
   partial character/settings edits, durable version lifecycle, atomic undo, frontend save barrier.
2. Prompt resources: search/detail/create/edit/copy/delete, complete main and character presets,
   resource concurrency, lexicon access, compiled preview and submission input consistency.
3. Generation feedback: opt-in output vision, model capability settings, output selection context,
   repeated generation, event-based waiting, cancellation and submission retry identity.

Each stage is verified and committed independently. No compatibility aliases for replaced tools.

## Verification

Run formatting, strict lint/type/build checks as applicable and pure-logic unit tests. The user's
testing boundary excludes integration, UI, end-to-end, performance and snapshot tests; do not run
the aggregate workspace/frontend test commands where they include those suites. Desktop interaction
and real provider behavior require user QA, with remaining checks recorded at delivery.

## Progress

- Stage 1: implemented. Passed Rust formatting, workspace strict clippy, line budget,
  frontend formatting/lint/typecheck/build, 17 targeted Rust pure-logic tests and 6 frontend
  pure-logic tests. Existing frontend warnings and non-failing line-budget/build warnings remain.
  Desktop QA pending: edit then immediately send; multi-field edit and undo; clear/recreate then
  attempt an old undo; failed turn refresh. Database failure injection is outside the allowed
  automated test scope; transactional behavior was reviewed in the adapter implementation.
- Stage 2: pending.
- Stage 3: pending.
