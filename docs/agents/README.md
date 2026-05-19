# agents 文档入口

## 本目录职责

`docs/agents/` 只保存早期架构思路、约束和取舍。当前阶段不把目录结构、crate 列表、前端框架或命令面定死。

建议阅读顺序：

1. `project-intent.md`
2. `architecture-guidance.md`
3. `inheritance-options.md`
4. `scaffold-decision.md`

## 当前共识

- 项目已进入最小可运行 scaffold 阶段，见 `scaffold-decision.md`。
- Rust 工作完成前必须跑通 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- pnpm 前端工作完成前必须跑通 `pnpm fmt:check`、`pnpm lint`、`pnpm test`；前端格式化与 lint 使用 Oxc 系列的 `oxfmt`、`oxlint`。
- NovelAI 对接优先交给 `novelai-bridge`。
- 新项目应吸收 `nait` 的产品经验，但不沿用其 `protocol -> gateway -> sdk -> core -> app -> tauri` 横向主链路。
- 后端采用 feature-first 方向：少量 `foundation/shared` 基础抽象，各 feature 自治，`kernel` 负责运行时状态与跨 feature 编排，`app/adapters` 负责 I/O、持久化和 Tauri-facing 收束。
- `kernel` 可以定义所需 ports，但不直接读写文件、数据库、keyring、HTTP 或 Tauri API。
- 模块拆分可以参考 `stringer` 的细粒度 workspace 风格，但不要机械复制 crate 数量。
- 前端可以重新评估 Solid 或 React；缓存与服务端状态策略应跟随 framework 选择。
- 任务调度、资源库、Prompt 工作区、Gallery、Vibe 管理应按 feature 独立演进。

## 非目标

- 现在不创建完整 feature crate 清单。
- 现在不实现 Prompt、generation、jobs、gallery 等业务 feature。
- 现在不承诺最终产品名、包名或 crate 名。
- 现在不迁移旧 `nait` 代码。
