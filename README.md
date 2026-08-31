# Atelier

Atelier is a desktop creative workspace for NovelAI image workflows.

The project is intentionally NovelAI-specific. It should preserve clear feature boundaries, use `novelai-bridge` for NovelAI API integration, and avoid turning into a generic AI image tool.

## Development

Prerequisites:

- Rust toolchain from `rust-toolchain.toml`
- Node.js from `.node-version`
- pnpm 11.24.0+

Common commands:

```powershell
pnpm install
pnpm dev
pnpm desktop:dev
pnpm build
pnpm fmt:check
pnpm lint
pnpm test
cargo check --workspace
cargo fmt --all -- --check
cargo clippy-strict
cargo test --workspace
cargo xtask line-budget
```

Before completing Rust work, confirm `cargo fmt --all -- --check`, `cargo clippy-strict`, `cargo test --workspace`, and `cargo xtask line-budget` pass.

Before completing pnpm frontend work, confirm `pnpm fmt:check`, `pnpm lint`, and `pnpm test` pass. Frontend formatting and linting use the Oxc toolchain: `oxfmt` and `oxlint`.

Application and downloadable-resource releases are manually started from GitHub Actions on
`main`, after CI passes for the exact source commit. Builds are saved separately from publication
so failed uploads can be retried without rebuilding. See [`docs/releasing.md`](docs/releasing.md).

## Repository Layout

- `apps/desktop/`: Tauri v2 shell and Vite React frontend.
- `crates/foundation/`: shared primitives that are stable across features.
- `crates/features/`: feature-owned domain models, rules, ports, and tests.
- `crates/kernel/`: runtime state and cross-feature workflow orchestration.
- `crates/adapters/`: concrete I/O implementations for storage, database, image codecs, application-level secret metadata, keyring, NovelAI, and optional safety scanning.
- `docs/agents/`: project intent and the small set of current architecture guidance documents.
- `xtask/`: local maintenance checks such as line-budget enforcement.

Tauri should stay thin, but it owns platform desktop host glue such as native dialogs, selected local file reads/writes, open/reveal guards, notifications, and bundled resource path resolution. Domain behavior belongs in feature crates, `kernel`, host-neutral application code, or adapters according to the architecture notes.
