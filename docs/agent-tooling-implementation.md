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
- Stage 2: implemented. Added complete observed resource maintenance, lexicon tools, registered
  prompt-function guidance, immutable compiled previews, resolved text/token usage/estimates and
  preview-bound submission approvals. Resource-library rewrites now advance durable draft versions.
  Passed 11 targeted Rust pure-logic tests and 4 frontend event-model tests, frontend formatting,
  lint, typecheck and build, plus Rust formatting, strict workspace clippy and line budget.
  Existing frontend and line-budget warnings remain. Desktop/provider QA is listed in `docs/agent-tools.md`.
  Preview invalidation intentionally covers the whole prompt library; draft undo does not undo
  resource mutations.
- Stage 3: implemented. Connected user output-vision controls and immutable model capability
  snapshots; reads resolve only output-area/turn-owned batch/job/sample references to native image
  tool content. Image payloads are omitted from event storage. Added event-based status/wait,
  host invocation identities, submission retry receipts, repeated preview-bound generation,
  batch-scoped cancellation and network cancellation for both ordinary and streaming requests.
  Queue workers retain pending starts and reject stale batch transitions. Runtime tool tasks drain
  their persistence boundary before cancellation cleanup; workspace leases remain held while an
  active Agent drains. Selected resource context and image provenance are carried between turns.
  Active-run context budgeting preserves text observations and omits older images with re-read notices.
  Passed 35 targeted Rust pure-logic tests and 4 frontend event-model tests, Rust formatting,
  workspace strict clippy and line budget, frontend formatting/lint/typecheck and production build.
  Existing frontend/line-budget/bundle warnings remain. No real provider or desktop interaction
  tests were run; required user QA is recorded in `docs/agent-tools.md`.

## Final scope audit

- Host observations replace model-written revisions, with stale-state feedback and atomic draft undo.
- Draft text/character/settings operations and exact replacement preserve omitted fields.
- Main/character presets and chunks support complete text, metadata, bindings and overrides.
- Preview, approval and submission share compiled inputs and bridge-resolved token/cost information.
- User-enabled vision reads actual available output pixels; no image guidance or pixel mutations.
- Iteration has fresh-preview gates, wait/status, host retry receipts and owned-batch cancellation.
- Deliberate limits: resource mutations are outside draft undo; preview invalidation covers the whole
  prompt library; session model capabilities are immutable; image/context estimates need provider QA.
