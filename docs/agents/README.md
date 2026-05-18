# agents 文档入口

## 本目录职责

`docs/agents/` 只保存早期架构思路、约束和取舍。当前阶段不把目录结构、crate 列表、前端框架或命令面定死。

建议阅读顺序：

1. `project-intent.md`
2. `architecture-guidance.md`
3. `inheritance-options.md`

## 当前共识

- NovelAI 对接优先交给 `novelai-bridge`。
- 新项目应吸收 `nait` 的产品经验，但不沿用其 gateway/sdk/core/app 主链路。
- 模块拆分可以参考 `stringer` 的细粒度 workspace 风格，但不要机械复制 crate 数量。
- 前端可以重新评估 Solid 或 React；缓存与服务端状态策略应跟随 framework 选择。
- 任务调度、资源库、Prompt 工作区、Gallery、Vibe 管理应按 feature 独立演进。

## 非目标

- 现在不创建完整 Rust workspace。
- 现在不生成 Tauri/Vite/Solid/React 工程。
- 现在不承诺最终产品名、包名或 crate 名。
- 现在不迁移旧 `nait` 代码。
