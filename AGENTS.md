# AGENTS.md

## Scope

This repository is the early implementation workspace for **Atelier**, a desktop creative workspace built specifically for NovelAI image workflows.

Keep the product NovelAI-focused. Do not drift toward a generic AI image platform. NovelAI API integration should default to the maintained `novelai-bridge` crate.

## Read First

Before design or implementation work, read:

1. `AGENTS.md`
2. `README.md`
3. `docs/agents/README.md`

Then read only the task-relevant design records listed from `docs/agents/README.md`.

## Working Rules

- Treat current docs as guidance, not frozen architecture contracts.
- Keep clear feature boundaries. Do not recreate a large horizontal `core`, `protocol`, or `ApplicationService`.
- Keep the Tauri shell thin.
- Real I/O belongs in adapters or desktop host glue, not in feature crates or `kernel`.
- Keep shared Rust dependency versions and local project crate path dependencies in root `[workspace.dependencies]`. Crate manifests should inherit them with `workspace = true`; `apps/desktop/src-tauri` may keep Tauri and desktop-host-only third-party dependencies local.
- `D:\Source\_Rust\nait` and `D:\Source\_Rust\stringer` are read-only reference projects.
- Do not copy reference project implementation without an explicit license and source record.

## Verification

Before completing Rust work, run and confirm:

```powershell
cargo fmt --all -- --check
cargo clippy-strict
cargo test --workspace
cargo xtask line-budget
```

Before completing pnpm frontend work, run and confirm:

```powershell
pnpm fmt:check
pnpm lint
pnpm test
```

## Documentation Style

- Prefer short decision notes and trade-off notes over speculative long specs.
- Keep public API, crate, type, command, and framework names in English.
