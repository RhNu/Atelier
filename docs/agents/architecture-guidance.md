# 架构指导

## 总原则

当前阶段固定架构方向，并记录后端 crate 布局设计方向；但不冻结最终 crate 名、持久化格式、前端框架或 Tauri command 名。

NAI Atelier 是对 `nait` 产品经验的重构，不继承 `nait` 的固定横向链路：

```text
protocol -> gateway -> sdk -> core -> app -> tauri
```

新项目采用 feature-first 架构：

```text
foundation -> features -> kernel
foundation -> app-api
features/kernel/app-api/adapters -> app
app -> Tauri shell / frontend
```

这是职责方向。具体后端 crate 布局见 `backend-crate-layout.md`，实现前仍可按真实复杂度决定边界是独立 crate 还是模块。

新项目应避免两类极端：

- 把所有领域能力堆进一个巨大的 `core` 或 `app` crate。
- 让 feature 自行创建长期资源目录、私有二进制索引和不可替换 I/O。

更合适的方式是先按 feature 定义边界，再由 `kernel` 组合运行时流程，由 host-neutral `app` 暴露 use case，最后由 `adapters` 集中实现 I/O、持久化和外部库接入。

## 架构层次

### `foundation`

`foundation` 只放稳定复用的基础抽象与跨 feature 原语。

可以进入这里：

- error envelope、diagnostic、event 基础结构。
- ID、time、受控路径、asset locator 等跨 feature 原语。
- 已经被两个以上 feature 稳定复用，且不属于某个 feature 语义核心的 helper。

不应进入这里：

- 完整 `GenerateImageParams`、Gallery item、Vibe document、Prompt resource catalog 等具体 feature DTO。
- 只被单个 feature 使用的 helper。
- 为了“以后可能复用”提前抽出来的模型。
- 让所有 feature 都依赖的巨型 `protocol` / `interface` 包。

提升规则：领域模型默认由 feature 拥有。提升到 shared 前必须说明复用者、边界收益、为什么不是 feature 私有 helper；无法说明时留在 feature 内。

### `features`

feature 是领域能力的默认归属。Prompt、generation、jobs、artifacts、resource catalog、prompt resources、gallery、Vibe、secrets、workspace 等能力应优先在 feature 内拥有自己的模型、规则、服务和测试。

feature 可以依赖 `foundation` 的基础抽象，也可以通过 `kernel` 参与跨 feature workflow，但不应把领域逻辑交给一个全局 service 代管。

feature 默认不做真实 I/O。需要持久化、网络、系统能力或外部运行库时，先定义 trait/port，再由 `adapters` 实现。

### `kernel`

`kernel` 是运行时编排内核，负责组织 feature 和维护应用运行状态。

`kernel` 可以：

- 组合 feature service。
- 维护运行中的 job、queue、selection、session、progress、cancellation、retry 等状态。
- 编排跨 feature workflow，例如 prompt 编译、Vibe 解析、generation 提交、artifact 注册、gallery 更新。
- 发出领域事件或应用事件。
- 定义它需要的 `port` trait，例如 `JobRepository`、`ArtifactRepository`、`NovelAiGenerationClient`、`SecretResolver`、`WorkspaceStore`。

`kernel` 不可以：

- 直接读写文件、数据库、redb/sqlite 或系统目录。
- 直接调用 `novelai-bridge`、HTTP client、keyring、clipboard、file dialog、notification 或 Tauri API。
- 承载具体持久化 schema 或迁移逻辑。
- 变成所有 feature 的唯一实现位置。

一句话规则：`kernel` owns orchestration and runtime state; `adapters` own I/O and persistence.

### `app-api`

`app-api` 定义前端客户端可见 contract，例如 request/response DTO、event DTO、error envelope、分页和 query DTO。它不是旧 `nait-protocol` 的复刻，不应成为所有 feature 内部模型的唯一来源。

### `app`

`app` 是 host-neutral application use case 层：

- 组织 settings、api key、work、resources、gallery、visual asset、director、vibe 等 use case group。
- 把 `app-api` DTO 转换为 feature/kernel input。
- 统一 runtime guard、错误映射、权限和预检策略。
- 构造并持有 `kernel` 与 adapter trait objects。

`app` 不依赖 Tauri API，不直接实现业务规则，不散落真实 I/O。

### `adapters`

`adapters` 实现 `kernel`、feature 或 `app` 所需 ports，并把外部能力接入应用：

- 文件系统、数据库、索引、cache、workspace root。
- `novelai-bridge` adapter。
- 系统凭据后端与 API key registry。
- image codec、NSFW runtime、系统能力，例如文件对话框、剪贴板、通知。

真实 I/O 只应出现在 adapters 或 desktop host adapter 中。测试时应能替换为 fake 或 in-memory adapter。

### `resource-catalog`

长期可持久资源必须通过统一 `resource-catalog` 接入。feature 不得各自创建资源目录、命名文件、维护私有二进制索引。

`resource-catalog` 负责：

- `ResourceId`、`ResourceKind`、`ResourceRef`、`ResourceRecord`、`ResourceVariant`。
- 资源 owner、metadata 边界、lifecycle。
- blob identity、hash、mime、尺寸、受控路径策略。
- 变体和 GC 的统一入口。

Gallery、Vibe、artifacts、Prompt thumb 等 feature 保存 `ResourceRef` 和自己的领域 metadata，不保存底层物理路径规则。

## 建议的边界问题

设计每个 feature 前先回答：

- 它提供什么用户可感知能力？
- 它拥有哪些数据？
- 它依赖哪些外部能力？
- 它是否需要独立测试、独立迁移、独立缓存或独立错误模型？
- 它暴露给前端的是命令、事件、查询，还是纯本地状态？

如果这些答案说不清，先不要拆成独立 crate。

## 后端 feature 候选

后端优先围绕这些 feature 边界思考；具体 crate 布局见 `backend-crate-layout.md`：

- `workspace`：项目目录、设置、路径、安全读写、锁。
- `settings`：应用设置、patch、validate、restart-required 字段。
- `secrets`：API key registry、active key、secret metadata、probe policy。
- `prompt`：Prompt 解析、格式化、诊断、函数描述。
- `prompt-lexicon`：词库 catalog、list/search 与匹配排序。
- `resource-catalog`：统一资源记录、blob、variant、owner 与 lifecycle。
- `prompt-resources`：chunk、preset、thumb binding、PromptTrace、orchestration compile。
- `generation`：生成提交、参数归一化、Anlas 估算、bridge 调用前后的转换。
- `jobs`：排队、取消、重试、事件、进度、并发策略。
- `artifacts`：产物语义、metadata、replay、visual asset contract、导出变体。
- `gallery`：产物索引、筛选、人工标记、跨 feature 入口。
- `vibe`：managed Vibe resource、encoding bucket、import/export、preview。
- `director`：Director tool 请求、输入校验、结果登记计划。
- `safety`：安全元数据、scan policy、manual override、risk band。

这些边界可以逐步落地，不要求一次性实现完整链路。

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
- 调用 `app` use case。
- 包装统一响应。
- 注册或调用 desktop host adapter，例如文件对话框、剪贴板、通知。

不要在 Tauri command 中实现任务调度、Prompt 编译、Gallery 聚合、NovelAI 请求构建、keyring registry 规则或资源索引规则。

## 文档策略

早期文档应偏短，记录正在形成的判断。等某个判断会影响长期实现成本时，再写 ADR 或更正式的 spec。
