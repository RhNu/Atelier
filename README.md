# NAI Atelier

NAI Atelier is an early desktop creative workspace for NovelAI image workflows.

The project is intentionally NovelAI-specific. It should preserve clear feature boundaries, use `novelai-bridge` for NovelAI API integration, and avoid turning into a generic AI image tool.

## Development

Prerequisites:

- Rust toolchain from `rust-toolchain.toml`
- Node.js 20.19+ or 22.12+
- pnpm 10.33+

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

## Repository Layout

- `apps/desktop/`: Tauri v2 shell and Vite React frontend.
- `crates/foundation/`: shared primitives that are stable across features.
- `crates/features/`: feature-owned domain models, rules, ports, and tests.
- `crates/kernel/`: runtime state and cross-feature workflow orchestration.
- `crates/adapters/`: concrete I/O implementations for storage, database, keyring, and NovelAI.
- `docs/agents/`: project intent, architecture overview, and decision records.
- `xtask/`: local maintenance checks such as line-budget enforcement.

Tauri should stay thin. Domain behavior belongs in feature crates, `kernel`, host-neutral application code, or adapters according to the architecture notes.
