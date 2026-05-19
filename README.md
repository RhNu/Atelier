# NAI Atelier

NAI Atelier 是面向 NovelAI 绘图工作流的早期桌面创作工作区。

## 开发

前置要求：

- `rust-toolchain.toml` 指定的 Rust toolchain
- Node.js 20.19+ 或 22.12+
- pnpm 10.33+

常用命令：

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

完成 Rust 相关工作前，必须确认 `cargo fmt --all -- --check`、`cargo clippy-strict`、`cargo test --workspace`、`cargo xtask line-budget` 通过。

完成 pnpm 前端相关工作前，必须确认 `pnpm fmt:check`、`pnpm lint`、`pnpm test` 通过。前端格式化和 lint 默认使用 Oxc 系列工具：`oxfmt` 与 `oxlint`。

## 目录

- `apps/desktop/`：Tauri v2 shell 与 Vite React 前端。
- `crates/foundation/`：极薄 Rust foundation crate，预留给稳定共享原语。
- `docs/agents/`：架构与 agent-facing 设计记录。

Tauri shell 应保持薄。领域行为只有在边界经过论证后，才进入 feature-owned Rust module 或 crate。
