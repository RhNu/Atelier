# Scaffold 决策记录

## 2026-05-19

NAI Atelier 进入最小可运行 scaffold 阶段。当前目标不是固定完整架构，而是把后续开发需要的仓库、包管理器和本地验证入口准备好。

已确定：

- Rust workspace 建立在仓库根目录。
- Tauri v2 与 Vite React 前端放在 `apps/desktop/`。
- 前端使用 pnpm workspace 与 Vite 官方 `react-ts` 模板。
- 前端格式化和 lint 使用 Oxc 系列工具：`oxfmt` 与 `oxlint`。
- Rust 侧只新增极薄 `crates/foundation/`，暂不接入 `src-tauri`。
- Tauri shell 保持薄，不在初始阶段暴露业务 command。

完成要求：

- Rust 工作完成前运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- pnpm 前端工作完成前运行 `pnpm fmt:check`、`pnpm lint`、`pnpm test`。

本记录只描述 scaffold 阶段决定。后续后端 crate 布局设计已进入 `backend-crate-layout.md`，以该文档为当前设计方向。

仍然不承诺：

- 持久化格式、Tauri command 命名或 typed IPC wire shape。
- 最终 crate 名、Prompt/generation/jobs/gallery 等 feature 的实现级模块边界。
