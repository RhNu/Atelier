# 架构指导

## 总原则

当前阶段固定架构方向，但不固定 crate 名、目录结构、持久化格式或 Tauri command 名。

NAI Atelier 是对 `nait` 产品经验的重构，不继承 `nait` 的固定横向链路：

```text
protocol -> gateway -> sdk -> core -> app -> tauri
```

新项目采用 feature-first 架构：

```text
foundation/shared -> features -> kernel -> app/adapters -> Tauri shell / frontend
```

这是一条职责方向，不是最终 crate 清单。后续可以按真实复杂度决定哪些边界需要独立 crate，哪些只需要模块。

新项目应避免两类极端：

- 把所有领域能力堆进一个巨大的 `core` 或 `app` crate。
- 过早创建大量 crate，导致每个边界都还没有真实职责却已经增加维护成本。

更合适的方式是先按 feature 定义边界，再由 `kernel` 组合运行时流程，最后在 `app/adapters` 层收束 I/O、持久化和 Tauri-facing 用例。

## 架构层次

### `foundation/shared`

`foundation/shared` 只放稳定复用的基础抽象与跨 feature 原语。

可以进入这里：

- error envelope、diagnostic、event 基础结构。
- ID、time、受控路径、asset locator 等跨 feature 原语。
- `port` trait 与 adapter contract。
- settings、secret、storage 这类基础能力接口。
- 已经被两个以上 feature 稳定复用，且不属于某个 feature 语义核心的 helper。

不应进入这里：

- 完整 `GenerateImageParams`、Gallery item、Vibe document、Prompt resource catalog 等具体 feature DTO。
- 只被单个 feature 使用的 helper。
- 为了“以后可能复用”提前抽出来的模型。
- 让所有 feature 都依赖的巨型 `protocol` / `interface` 包。

提升规则：领域模型默认由 feature 拥有。提升到 shared 前必须说明复用者、边界收益、为什么不是 feature 私有 helper；无法说明时留在 feature 内。

### `features`

feature 是领域能力的默认归属。Prompt、generation、jobs、artifacts、resources、gallery、Vibe、secret、workspace 等能力应优先在 feature 内拥有自己的模型、规则、服务和测试。

feature 可以依赖 `foundation/shared` 的基础抽象，也可以通过 `kernel` 参与跨 feature workflow，但不应把领域逻辑交给一个全局 service 代管。

### `kernel`

`kernel` 是运行时编排内核，负责组织 feature 和维护应用运行状态。

`kernel` 可以：

- 组合 feature service。
- 维护运行中的 job、queue、selection、session、progress、cancellation、retry 等状态。
- 编排跨 feature workflow，例如 prompt 编译、Vibe 解析、generation 提交、artifact 注册、gallery 更新。
- 发出领域事件或应用事件。
- 定义它需要的 `port` trait，例如 `JobRepository`、`ArtifactStore`、`NovelAiAdapter`、`SecretProvider`、`WorkspaceStore`。

`kernel` 不可以：

- 直接读写文件、数据库、redb/sqlite 或系统目录。
- 直接调用 `novelai-bridge`、HTTP client、keyring、clipboard、file dialog、notification 或 Tauri API。
- 承载具体持久化 schema 或迁移逻辑。
- 变成所有 feature 的唯一实现位置。

一句话规则：`kernel` owns orchestration and runtime state; `app/adapters` own I/O and persistence.

### `app/adapters`

`app/adapters` 实现 `kernel` 所需 ports，并把外部能力接入应用：

- 文件系统、数据库、索引、cache、workspace root。
- `novelai-bridge` adapter。
- 系统凭据后端与 API key registry。
- Tauri-facing 用例收束。
- 系统能力，例如文件对话框、剪贴板、通知。

这里可以编排 Tauri-facing use case，但不应重新长成一个全能 `ApplicationService`。当某个 use case 开始承载稳定领域规则，应把规则下沉到对应 feature 或 `kernel` workflow。

## 建议的边界问题

设计每个 feature 前先回答：

- 它提供什么用户可感知能力？
- 它拥有哪些数据？
- 它依赖哪些外部能力？
- 它是否需要独立测试、独立迁移、独立缓存或独立错误模型？
- 它暴露给前端的是命令、事件、查询，还是纯本地状态？

如果这些答案说不清，先不要拆成独立 crate。

## 后端 feature 候选

后端优先围绕这些 feature 边界思考，但不要先把它们写成固定 crate 清单：

- `workspace`：项目目录、设置、路径、安全读写、锁。
- `secret`：API key registry 与系统凭据后端。
- `novelai` adapter：把应用领域请求转换为 `novelai-bridge` 请求。
- `generation`：生成提交、参数归一化、Anlas 估算、bridge 调用前后的转换。
- `jobs`：排队、取消、重试、事件、进度、并发策略。
- `artifacts`：产物落盘、metadata、replay、导出变体、路径约束。
- `prompt`：Prompt 解析、格式化、诊断、函数描述。
- `resources`：chunk、preset、thumb、lexicon、Vibe 资源。
- `gallery`：产物索引、筛选、人工标记、跨 feature 入口。

这些边界可以逐步演化。第一版可以少一些，等职责变重再拆。

## 前端方向

如果选择 Solid 或 React，优先按 feature 组织，而不是按技术类型铺开：

```text
features/<feature>/
  data/        # query, mutation, cache invalidation
  model/       # local state, derived state, adapters
  ui/          # feature-owned components
  commands.ts  # typed IPC facade if needed
```

跨 feature 共享的东西要少而稳定：基础 UI、typed IPC、路径/资产工具、错误显示、i18n、通知。

## Tauri 边界

Tauri shell 应保持薄：

- 反序列化 command 参数。
- 调用 app service。
- 包装统一响应。
- 处理系统能力，例如文件对话框、剪贴板、通知、keyring。

不要在 Tauri command 中实现任务调度、Prompt 编译、Gallery 聚合或 NovelAI 请求构建。

## 文档策略

早期文档应偏短，记录正在形成的判断。等某个判断会影响长期实现成本时，再写 ADR 或更正式的 spec。
