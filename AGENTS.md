# AGENTS.md

## 范围

本仓库是 **NAI Atelier** 的早期设计工作区。

项目目标是成为专门面向 NovelAI 的桌面创作工作区。设计上应优先保持清晰 feature 边界，用已维护发布的 `novelai-bridge` crate 接管 NovelAI API 对接，并持续强调 NovelAI 专用定位，避免漂移成泛 AI 图像工具。

## 必读顺序

开始设计或实现前，先阅读：

1. `AGENTS.md`
2. `docs/agents/README.md`
3. `docs/agents/project-intent.md`
4. `docs/agents/architecture-guidance.md`
5. `docs/agents/backend-crate-layout.md`
6. `docs/agents/backend-rollout-plan.md`
7. `docs/agents/inheritance-options.md`
8. `docs/agents/scaffold-decision.md`
9. `docs/agents/kernel-generation-workflow.md`

## 工作规则

- 当前文档是指导，不是冻结的架构契约。
- 未经过单独设计论证前，不固定 crate 名、前端框架、持久化格式或 command 名。
- 每次 Rust 工作完成前必须运行并确认 `cargo fmt --all -- --check`、`cargo clippy-strict`、`cargo test --workspace`、`cargo xtask line-budget`。
- 每次 pnpm 前端工作完成前必须运行并确认 `pnpm fmt:check`、`pnpm lint`、`pnpm test`。
- 优先按 feature 划边界，避免大型横向层把领域逻辑耦合到一起。
- 引入桌面壳时，Tauri shell 保持薄。
- NovelAI API 集成默认使用 `novelai-bridge`。
- `D:\Source\_Rust\nait` 与 `D:\Source\_Rust\stringer` 只作为只读参考项目。
- 未写明许可证与来源记录前，不复制参考项目实现。

## 文档风格

- 主文档使用中文。
- Public API、crate、type、command、framework 名称保持英文。
- 优先写短 decision note 与 trade-off note，避免在早期堆长篇 speculative spec。
