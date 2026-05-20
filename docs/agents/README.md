# Agent Docs

This directory keeps project intent, architecture guidance, and decision records for NAI Atelier.

## Read Guide

Default required reading is intentionally short:

1. `AGENTS.md`
2. `README.md`
3. `docs/agents/README.md`

For architecture or backend work, also read:

1. `project-intent.md`
2. `architecture.md`

For task-specific context, read only the relevant decision records:

- `kernel-generation-workflow.md`: implemented `kernel` generation, streaming, Vibe, and precise-reference workflow boundaries.
- `sqlite-database-adapter.md`: SQLite adapter choice, schema scope, and adapter-local DTO boundary.
- `secrets-keyring-pipeline.md`: API key registry, keyring storage, explicit subscription probe, and resolver-backed NovelAI adapter.
- `prompt-lexicon-workflow.md`: prompt lexicon source assets, Rust `xtask` build/check workflow, v1 generated schema, and `nait` source notes.
- `safety-onnx-adapter.md`: host-provided OpenNSFW ONNX model/runtime loading, scanner injection, and license/source boundary.

## Current Consensus

- NAI Atelier is a NovelAI-specific desktop creative workspace.
- Feature crates own domain models, rules, ports, and tests.
- `kernel` owns runtime state and cross-feature orchestration, but not real I/O.
- Adapters own filesystem, database, keyring, `novelai-bridge`, and other external integrations.
- The Tauri shell is a desktop host adapter, not the application layer.
- Long-lived binary or semi-structured resources go through `resource-catalog`.
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
- `crates/features/vibe`
- `crates/features/director`
- `crates/features/safety`
- `crates/features/secrets`
- `crates/features/precise-reference`
- `crates/features/settings`
- `crates/adapters/storage-fs`
- `crates/adapters/database`
- `crates/adapters/image-codec`
- `crates/adapters/keyring`
- `crates/adapters/novelai`
- `crates/adapters/safety-onnx`

Still planned or intentionally not present:

- `adapters/desktop-system`

## Non-Goals

- Do not migrate old `nait` code.
- Do not preserve old `nait` command names or DTOs by default.
- Do not define completion as "can generate one image"; preserve the backend boundaries first.
