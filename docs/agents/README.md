# Agent Docs

This directory keeps the small set of long-lived guidance documents for Atelier. Implementation
details belong in the code and tests; source and license provenance belongs in `CREDITS.md` and the
relevant asset directories.

## Read Guide

Default required reading is intentionally short:

1. `AGENTS.md`
2. `README.md`
3. `docs/agents/README.md`

For architecture or backend work, also read:

1. `project-intent.md`
2. `architecture.md`

For frontend work in `apps/desktop`, also read:

1. `frontend-architecture.md`

The retained documents are:

- `project-intent.md`: product boundary, durable design preferences, and non-goals.
- `architecture.md`: backend layering, ownership, I/O boundaries, and current workspace shape.
- `frontend-architecture.md`: frontend boundaries, state ownership, visual system, and testing rules.
- `prompt-preset-model.md`: current prompt-preset persistence and editor model decision.

Do not add short-lived implementation plans or feature snapshots here. When a decision becomes a
durable constraint, update the closest retained document or add a focused record only if it cannot
be expressed there.

## Current Consensus

- Atelier is a NovelAI-specific desktop creative workspace.
- Feature crates own domain models, rules, ports, and tests.
- `kernel` owns runtime state and cross-feature orchestration, but not real I/O.
- Adapters own filesystem, database, keyring, `novelai-bridge`, and other external integrations.
- The Tauri shell is a desktop host adapter, not the application layer; it owns platform desktop host glue and keeps file picker reads/writes out of the frontend.
- Workspace-owned creative resources go through `resource-catalog`; global, reconstructable
  runtime assets go through `downloadable-resources`.
- Reference projects are read-only inputs. Do not copy implementation without license and source records.

## Current Implementation Shape

Implemented or partially implemented:

- `crates/foundation`
- `crates/kernel`
- `crates/app`
- `crates/app-api`
- `crates/features/workspace`
- `crates/features/resource-catalog`
- `crates/features/prompt`
- `crates/features/prompt-resources`
- `crates/features/prompt-lexicon`
- `crates/features/generation`
- `crates/features/jobs`
- `crates/features/artifacts`
- `crates/features/gallery`
- `crates/features/explore`
- `crates/features/danbooru`
- `crates/features/image-analysis`
- `crates/features/vibe`
- `crates/features/director`
- `crates/features/downloadable-resources`
- `crates/features/safety`
- `crates/features/secrets`
- `crates/features/precise-reference`
- `crates/features/settings`
- `crates/adapters/storage-fs`
- `crates/adapters/database`
- `crates/adapters/image-codec`
- `crates/adapters/keyring`
- `crates/adapters/secrets-fs`
- `crates/adapters/novelai`
- `crates/adapters/novelai-explore`
- `crates/adapters/danbooru`
- `crates/adapters/image-analysis-onnx`
- `crates/adapters/downloadable-resources-fs`
- `crates/adapters/settings-fs`

## Non-Goals

- Do not migrate old `nait` code.
- Do not preserve old `nait` command names or DTOs by default.
- Do not define completion as "can generate one image"; preserve the backend boundaries first.
