# agents 文档入口

## 本目录职责

`docs/agents/` 保存早期架构思路、约束和取舍。当前已经形成后端 crate 布局设计方向，但 crate 名、持久化格式、前端框架和 Tauri command 面仍需在实现前逐项确认。

建议阅读顺序：

1. `project-intent.md`
2. `architecture-guidance.md`
3. `backend-crate-layout.md`
4. `inheritance-options.md`
5. `scaffold-decision.md`

## 当前共识

- 项目已进入最小可运行 scaffold 阶段，见 `scaffold-decision.md`。
- Rust 工作完成前必须跑通 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- pnpm 前端工作完成前必须跑通 `pnpm fmt:check`、`pnpm lint`、`pnpm test`；前端格式化与 lint 使用 Oxc 系列的 `oxfmt`、`oxlint`。
- NovelAI 对接优先交给 `novelai-bridge`。
- 新项目应吸收 `nait` 的产品经验，但不沿用其 `protocol -> gateway -> sdk -> core -> app -> tauri` 横向主链路。
- 后端采用 feature-first 方向：少量 `foundation` 基础抽象，各 feature 自治，`kernel` 负责运行时状态与跨 feature 编排，`app` 负责 host-neutral use case，`adapters` 集中实现 I/O。
- Tauri 是当前 desktop host adapter，不是 application layer；`app` 不依赖 Tauri API。
- `kernel` 可以定义所需 ports，但不直接读写文件、数据库、keyring、HTTP 或 Tauri API。
- 资源管理通过统一 `resource-catalog` 抽象接入，feature 不自行长期创建资源目录或维护私有二进制索引。
- 模块拆分可以参考 `stringer` 的细粒度 workspace 风格，但每个 crate 必须有明确职责、依赖方向和测试替换边界。
- 前端可以重新评估 Solid 或 React；缓存与服务端状态策略应跟随 framework 选择。
- 任务调度、Prompt 工作区、Gallery、Vibe 管理应按 feature 独立演进，并通过 trait/port 与集中 adapters 连接真实 I/O。

## 非目标

- 现在不实现 Prompt、generation、jobs、gallery 等业务 feature。
- 现在不承诺最终产品名、包名或 crate 名。
- 现在不迁移旧 `nait` 代码。
