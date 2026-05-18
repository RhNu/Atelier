# 可继承方案与取舍

## 定位

NAI Atelier 是对 `nait` 产品经验的重构，不是对 `nait` 分层架构的延续。

`nait` 已经验证了大量 NovelAI 桌面工作流细节，但它的固定横向链路和聚合服务在迭代后变得过重。新项目应继承已验证的产品判断、安全边界和交互经验，同时反继承容易导致大 `core`、大 `app`、大 `protocol` 的架构模式。

## 从 `nait` 继承

### 可以继承的思路

- Tauri shell 保持薄，业务逻辑不要写进 command。
- API key secret 只进入系统凭据后端，前端和普通 JSON 文件只保存非敏感元数据。
- 前端通过 typed IPC 调用后端，不散落 raw invoke。
- Prompt 解析、诊断和词库检索由后端提供，前端只处理编辑器交互。
- 受控相对路径是可视资产解析与导出的基础。
- `work/event` 一类事件通道适合承载任务进度和通知。
- Prompt、Gallery、Vibe、Director、多 key 等产品经验可以作为行为参考。
- artifact replay、batch/job 事件、可视资产导出等已验证概念可以重新设计后继承。

### 不建议继承的部分

- `nait-gateway` / `nait-sdk` 内置 NovelAI 链路；新项目应交给 `novelai-bridge`。
- 固定横向链路 `protocol -> gateway -> sdk -> core -> app -> tauri`。
- 过重的 `ApplicationService` 门面。
- 一个 `WorkKernel` 同时承担调度、存储、Gallery、replay、orchestration、Vibe 的模式。
- 巨型 `protocol` / `interface` 承载所有 feature DTO 的模式。
- 前端领域代码同时散落在 `features`、`composables`、`components/workbench` 的形态。
- 以 `.ref` 对齐作为长期文档负担。

### Pros

- 已经验证过 NovelAI 桌面工具的大量产品细节。
- Prompt、Gallery、Vibe、Director、多 key 等领域问题都有现成经验。
- 安全边界和 Tauri 边界比较成熟。

### Cons

- 历史包袱重，很多边界是演进结果，不一定适合新项目直接继承。
- 横向分层链路太强，feature 自治不足。
- 大文件和大服务已经说明部分职责拆分不够早。

### 新项目的替代判断

- 用 feature-first 边界取代固定横向链路。
- 保留 `kernel` 作为运行时编排内核，但 `kernel` 不直接做 I/O 或持久化。
- 由 `app/adapters` 实现 `kernel` ports，并接入文件系统、数据库、`novelai-bridge`、keyring 与系统能力。
- 领域模型默认归属 feature，只有稳定跨 feature 复用时才提升到 `foundation/shared`。

## 从 `novelai-bridge` 继承

### 可以继承的思路

- NovelAI 网络请求、错误映射、`Retry-After`、SSE、PNG metadata、Vibe helper 由独立 crate 负责。
- 应用层只做产品语义，不直接拼 NovelAI wire payload。
- 使用 `Transport` trait 保留测试和替换空间。
- Prompt 语法不放进 bridge，避免 API client 变成应用内核。

### Pros

- 明确降低新项目的 NovelAI 对接维护成本。
- 可以让新项目把精力放在 workflow、资源、UI 和任务系统。
- 已是发布 crate，更适合作为稳定依赖边界。

### Cons

- bridge 的错误模型是 Rust-native enum，Tauri 公共错误 envelope 需要应用层转换。
- 如果产品需要 token 级调度、冷却、自动轮换，仍需要 app feature 自己设计。

## 从 `stringer` 继承

### 可以继承的思路

- 细粒度 workspace，但每个 crate 都要有清楚职责。
- 可以参考 `interface` crate 的共享契约思路，但新项目不默认建立巨型 DTO 中心。
- `app` crate 做用例路由，入口文件保持轻。
- `xtask line-budget` 这类机械守卫能提前压住文件膨胀。
- 工作流文档和 agent 指南可以放在项目内，服务长期维护。

### Pros

- 模块边界更接近长期维护项目。
- 适合把任务、资源、知识库、导入导出等能力独立演化。
- 行数预算能防止新项目快速回到大文件状态。

### Cons

- 如果一开始照搬 crate 数量，会增加启动成本。
- Stringer 是 CLI/workspace 工具，不是 Tauri 图形应用；前端和桌面交互经验不能直接套。
- 它的工作区模型偏文件批处理，NAI Workspace 的实时生成任务需要另行设计。

## 前端框架选项

### Solid

Pros:

- 细粒度响应式适合复杂编辑器和参数面板。
- 本地状态模型轻，性能开销低。
- 对高频 UI 更新友好。

Cons:

- 生态、组件库、测试资料少于 React。
- 团队或 agent 生成代码时更容易写出不符合 Solid 心智模型的代码。
- Tauri 桌面 app 的现成范式相对少。

### React

Pros:

- 生态成熟，TanStack Query、Router、表单、虚拟列表、测试工具选择多。
- Agent 与开发者都更熟悉，长期维护风险低。
- 复杂 UI 拆分、设计系统和 Storybook 类工具支持更好。

Cons:

- 默认渲染模型对复杂编辑器和高频状态需要更谨慎的结构设计。
- 容易把状态和 effects 写散，需要明确 feature 边界和 lint/架构守卫。

### 暂定建议

如果目标是尽快建立长期可维护桌面产品，React 更稳。如果目标是探索更轻、更响应式的创作工作台，Solid 值得试验。两者都不应在没有小型 spike 前定死。
