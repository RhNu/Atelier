# 后端 crate 布局设计

## 状态

- Date: 2026-05-19
- Status: Design direction

本文记录 NAI Atelier 后端的具体 crate 布局方向。它用于指导后续逐块实现，不要求一次性打通最短链路，也不把当前名称冻结成永久契约。实现前仍可按真实复杂度微调 crate 名、目录名、持久化格式和 Tauri command 名。

## 目标

NAI Atelier 后端要完整承接 `nait` 已验证的大部分 NovelAI 桌面创作能力，但不继承 `nait` 的横向主链路：

```text
protocol -> gateway -> sdk -> core -> app -> tauri
```

新的维护目标是：

- 优先防止大 `core`、大 `app`、大 `ApplicationService`。
- 通过 trait/port 隔离真实 I/O，使测试可以随意替换 filesystem、database、keyring、NovelAI client、NSFW scanner、clipboard 和 notification。
- 保持 NovelAI 专用领域语言，不抽象成泛 AI 生成平台。
- 不设计 MVP 半成品；按完整后端能力做边界设计，再逐个 feature 落地。
- 可以接受较细 crate 拆分，只要每个 crate 有清楚职责、依赖方向和测试边界。

## 非目标

- 不复制 `nait` 实现。
- 不兼容旧 `nait` command 名或 DTO。
- 不在本文确定数据库、文件格式、TS 类型生成工具或最终 Tauri command 命名。
- 不把桌面 Tauri host 作为后端架构所有者。

## 总体形态

后端分为三条主线：

1. **Domain / feature crates**
   拥有 NovelAI 创作领域模型、规则和纯服务。
2. **Kernel**
   拥有运行时状态、任务队列、事件、取消、重试和跨 feature workflow。
3. **App + adapters**
   `app` 暴露 host-neutral application use cases；`adapters` 集中实现真实 I/O。

Tauri 只作为当前桌面 host：

```text
frontend client
  -> transport commands/events
  -> Tauri shell
  -> app use cases
  -> kernel workflows
  -> feature services
  -> ports
  -> adapters / I/O
```

硬约束：

- Tauri is a host adapter, not the application layer.
- `app` 不依赖 `tauri` crate。
- `kernel` 不直接读写文件、数据库、keyring、HTTP、clipboard、file dialog 或 notification。
- `feature` 默认不做真实 I/O。
- `adapters` 是真实 I/O 唯一长期落点。

## 建议 workspace 结构

```text
crates/
  foundation/
  app-api/
  app/

  kernel/

  features/
    workspace/
    settings/
    secrets/
    prompt/
    prompt-lexicon/
    resource-catalog/
    prompt-resources/
    generation/
    jobs/
    artifacts/
    gallery/
    vibe/
    director/
    safety/

  adapters/
    storage-fs/
    database/
    keyring/
    novelai/
    image-codec/
    safety-onnx/
    desktop-system/

apps/
  desktop/src-tauri/
```

依赖方向：

```text
foundation -> features -> kernel
foundation -> app-api
features/kernel/app-api/adapters -> app
app -> apps/desktop/src-tauri
```

`adapters` 依赖 `foundation`、相关 feature/kernel ports 和外部库。`app` 在构造阶段组合 `kernel` 与 adapters。Tauri 依赖 `app` 与 `app-api`，并注册 desktop host adapters。`kernel` 不依赖 `app-api`，`adapters` 不依赖 `app`。

`app-api` 不是旧 `nait-protocol` 的复刻。它只放客户端可见 contract：request/response DTO、event DTO、error envelope、分页/query DTO。领域内部模型仍归 feature crate 所有。

## 核心 crate 职责

### `foundation`

职责：

- 稳定基础原语：`Id`、`Timestamp`、`RelativePath`、`ControlledPath`、`ByteSize`、`ContentHash`。
- 通用 error、diagnostic、event 基础结构。
- async port 辅助类型，例如 `DynResult<T>`、`BoxStream<T>`。
- 脱敏 helper 与路径越界检查基础工具。

不做：

- 不放 `GenerateImageParams`、`GalleryItem`、`PromptPreset`、`VibeResource`。
- 不定义业务 use case。
- 不依赖 Tauri、database、keyring 或 `novelai-bridge`。

### `app-api`

职责：

- 定义前端客户端可见的稳定 contract。
- 按 domain 分模块：`runtime`、`settings`、`api_keys`、`subscription`、`work`、`gallery`、`visual_asset`、`prompt`、`resources`、`vibe`、`director`。
- 提供后续 TS 类型导出入口。

不做：

- 不承载 feature 内部模型的唯一来源。
- 不依赖 `app`、`kernel`、Tauri 或 adapters。

### `kernel`

职责：

- 运行时状态：`booting | ready | failed`、session、event hub。
- job queue、concurrency、cancel token、retry policy、progress/event stream。
- 跨 feature workflow：
  - Prompt/resources compile。
  - generation submit。
  - artifact persist request。
  - gallery index update。
  - replay payload assembly。
  - Vibe encode/cache workflow。
  - Director result registration。
- 定义编排需要的 ports，例如 `GenerationClient`、`JobRepository`、`ArtifactStore`、`GalleryIndex`、`WorkspaceStore`、`SecretResolver`、`SafetyScanner`、`Clock`、`EventSink`。

不做：

- 不直接读写文件、数据库、keyring、clipboard。
- 不调用 `novelai-bridge`。
- 不拥有 Prompt parser、Vibe document、Gallery query 规则的内部实现。
- 不暴露 Tauri command。

### `app`

职责：

- 面向客户端组织 use case facade：`GenerateUseCases`、`ResourceUseCases`、`GalleryUseCases`、`SettingsUseCases`、`ApiKeyUseCases`。
- 把 `app-api` DTO 转换为 feature/kernel input。
- 统一错误映射、runtime guard、权限/预检策略。
- 构造并持有 `kernel` 与 adapters。
- 提供 host-neutral `AppRuntime` / `AppState`。

不做：

- 不实现业务规则。
- 不直接出现 Tauri API 类型。
- 不散落文件系统或 keyring 读写。
- 不变成单个全能 `ApplicationService`。

### `apps/desktop/src-tauri`

职责：

- command 参数反序列化。
- 调用 `app` use case。
- 包装 `app-api` envelope。
- 订阅 app event 并转发 Tauri event。
- 注册 file dialog、clipboard、notification、app data dir、Tauri event emit 等 desktop host adapters。

不做：

- 不创建第二套 runtime 状态机。
- 不直接读写 workspace。
- 不直接调 feature/kernel/adapters 的内部 API。
- 不直接依赖 `novelai-bridge`。

## feature crates

### `features/workspace`

职责：workspace root、受控目录、路径策略、锁、版本标记、目录 layout。  
模块：`layout`、`paths`、`lock`、`version`、`ports`。  
ports：`WorkspaceStore`、`WorkspaceLock`。  
不做：不管理具体 gallery/job/vibe schema。

### `features/settings`

职责：应用设置模型、patch/validate、restart-required 字段判定。  
模块：`model`、`patch`、`validation`、`ports`。  
ports：`SettingsStore`。  
不做：不读写 JSON，不碰前端 i18n。

### `features/secrets`

职责：API key registry 领域规则、active key 切换、secret metadata、probe policy。  
模块：`registry`、`active_key`、`subscription_probe`、`ports`。  
ports：`SecretStore`、`ApiKeyRegistryStore`、`SubscriptionClient`。  
不做：不调用 keyring，不存明文 secret。

### `features/prompt`

职责：Prompt parser、formatter、diagnostics、function descriptor、span 语义。  
模块：`parser`、`formatter`、`diagnostics`、`functions`、`model_gate`。  
不做：不管理 chunk/preset，不做资源编译。

### `features/prompt-lexicon`

职责：Lexicon catalog、list/search、匹配 rank、translation metadata。  
模块：`catalog`、`search`、`provider`、`index`。  
ports：`LexiconProvider`。  
不做：不让前端直接读词库文件。

### `features/resource-catalog`

职责：统一管理所有可落盘、可索引、可引用、可清理的二进制/半结构化资源。  
模块：`kind`、`record`、`blob`、`variant`、`owner`、`lifecycle`、`ports`。  
ports：`ResourceCatalogRepository`、`ResourceBlobStore`、`ResourceVariantBuilder`。  
不做：不拥有 Gallery query、Vibe encoding、Prompt compile 等领域语义。

核心类型：

- `ResourceId`
- `ResourceKind`
- `ResourceRecord`
- `ResourceBlob`
- `ResourceVariant`
- `ResourceRef`
- `ResourceOwner`
- `ResourceMetadata`
- `ResourceLifecycle`

典型 `ResourceKind`：

- `generated_image`
- `stream_final_image`
- `director_result`
- `source_image`
- `reference_image`
- `controlnet_image`
- `prompt_thumb`
- `vibe_document`
- `vibe_preview`
- `vibe_encoding`
- `lexicon_bundle`

硬约束：

- 任何 feature 不得直接创建长期资源目录或维护私有二进制索引。
- 新增可持久资源必须先定义 `ResourceKind`、metadata 边界、owner 和 lifecycle，再通过 `resource-catalog` 接入。
- 物理路径按 kind/date/hash 分桶是 adapter 实现细节，不泄漏给 feature。

### `features/prompt-resources`

职责：chunk、preset、thumb binding、Prompt orchestration compile、PromptTrace。  
模块：`chunk`、`preset`、`thumb`、`compiler`、`trace`、`ports`。  
ports：`PromptResourceRepository`。  
依赖：`prompt`、`resource-catalog`。  
不做：不提交 generation，不保存最终产物文件。

### `features/generation`

职责：NovelAI generation 领域参数、参数归一化、Anlas 估算、request plan、bridge 前后的应用语义转换。  
模块：`params`、`normalize`、`estimate`、`request_plan`、`models`、`ports`。  
ports：`NovelAiGenerationClient`、`NovelAiStreamingClient`、`SubscriptionClient`。  
不做：不拥有队列，不直接落盘。

### `features/jobs`

职责：job/batch 状态机、retry/cancel policy、progress model、event model。  
模块：`job`、`batch`、`state_machine`、`retry`、`events`、`ports`。  
ports：`JobRepository`、`JobEventSink`。  
不做：不决定 NovelAI payload 细节，不保存图片文件。

### `features/artifacts`

职责：产物模型、metadata、replay manifest、visual asset contract、导出变体语义。  
模块：`artifact`、`metadata`、`replay`、`visual_asset`、`transfer`、`ports`。  
ports：`ArtifactRepository`、`ImageTranscoder`。  
依赖：`resource-catalog`。  
不做：不管理 Gallery query，不调用 clipboard，不决定资源物理路径。

### `features/gallery`

职责：Gallery item、索引查询、筛选排序、人工安全标记、跨来源聚合规则。  
模块：`item`、`query`、`index`、`safety_override`、`ports`。  
ports：`GalleryIndex`。  
依赖：`resource-catalog`、`artifacts`、`safety`。  
不做：不保存原图字节，不做 NSFW 推理。

### `features/vibe`

职责：managed Vibe resource、import/export、model encoding bucket、preview、hide/rename。  
模块：`document`、`resource`、`encoding`、`cache`、`import_export`、`ports`。  
ports：`VibeRepository`、`VibeCodec`、`NovelAiVibeClient`。  
依赖：`resource-catalog`。  
不做：不把 Vibe 当泛 embedding 系统，不自建 cache 目录。

### `features/director`

职责：Director tool 请求领域模型、输入图校验、结果登记计划。  
模块：`tool`、`input`、`request_plan`、`ports`。  
ports：`NovelAiDirectorClient`。  
依赖：`resource-catalog`。  
不做：不进入 job queue，除非以后明确要把 Director 纳入任务系统。

### `features/safety`

职责：安全元数据模型、scan policy、manual override 合成、risk band。  
模块：`model`、`policy`、`scanner`、`override`、`ports`。  
ports：`SafetyScanner`。  
不做：不绑定 ONNX Runtime，不决定前端模糊展示。

## adapters

### `adapters/storage-fs`

职责：

- workspace 目录创建、受控路径解析、文件读写、原子写入、导入/导出文件。
- resource blob 的实际文件布局。
- 路径越界、绝对路径、`..`、非允许目录的防线。

实现 ports：`WorkspaceStore`、`ResourceBlobStore`、`ReferenceCacheStore`。

### `adapters/database`

职责：

- 持久化索引和结构化数据。
- settings、API key registry metadata、job/batch index、gallery index、prompt resources、resource catalog、vibe metadata。
- schema version、migration、backup/repair 策略。

实现 ports：`SettingsStore`、`ApiKeyRegistryStore`、`JobRepository`、`GalleryIndex`、`PromptResourceRepository`、`ResourceCatalogRepository`、`VibeRepository`。

具体选型，例如 `redb` 或 SQLite，后续单独决策。

### `adapters/keyring`

职责：

- 系统凭据后端读写 API key secret。
- secret record id 与 registry metadata 对接。
- 后端不可用时返回统一 `storage_error`。

实现 ports：`SecretStore`。

不做：不保存 alias，不决定 active key 策略，不暴露 secret 给前端。

### `adapters/novelai`

职责：

- 唯一依赖 `novelai-bridge` 的 crate。
- 把 `generation`、`vibe`、`director`、`secrets` 所需 port 映射到 bridge client。
- 负责 NovelAI 错误到应用错误的边界转换，包括 `Retry-After`、network、auth、rate limit。
- 维护 bridge 能力适配表，避免应用层追随 wire model。

实现 ports：`NovelAiGenerationClient`、`NovelAiStreamingClient`、`NovelAiVibeClient`、`NovelAiDirectorClient`、`SubscriptionClient`。

不做：不实现 API key manager，不做 job retry policy，不把 bridge 类型向上泄漏到 feature/app-api。

### `adapters/image-codec`

职责：

- PNG metadata 读取/写入/清理。
- `original_png`、`sanitized_png`、`jpg` 转码。
- 透明像素白底合成。
- image dimensions、mime sniffing、hash。

实现 ports：`ImageInspector`、`ImageTranscoder`、`PngMetadataCodec`、`ResourceVariantBuilder`。

### `adapters/safety-onnx`

职责：

- ONNX Runtime 装载、模型资源发现、CPU 推理。
- 图片预处理、score 输出、降级 warning。
- 只对最终持久化图片扫描。

实现 ports：`SafetyScanner`。

不做：不决定人工 override，不决定前端模糊策略。

### `adapters/desktop-system`

职责：

- 当前 desktop host 才能提供的 file dialog、clipboard、notification、app data dir、Tauri event emit。
- 通过 trait 接入 `app`，不让 `app` 依赖 Tauri type。

实现 ports：`FileDialog`、`ClipboardWriter`、`NotificationSink`、`HostEventEmitter`、`AppDirectoryProvider`。

## app 与 Tauri 的关系

`app` 是 application runtime owner：

- 构造 use case groups。
- 持有 `kernel`。
- 注入 adapters。
- 统一 runtime guard。
- 输出 host-neutral responses/events。

Tauri 是 host transport owner：

- 把 command/event 映射到 `app-api` contract。
- 注册 `TauriClipboardWriter`、`TauriNotificationSink` 等 desktop adapters。
- 不跳过 `app` 调 feature。

概念形状：

```rust
// app crate owns this shape conceptually
pub struct AppRuntime<P> {
    use_cases: UseCases,
    ports: P,
}

// desktop host owns only this wiring conceptually
let runtime = AppRuntimeBuilder::new()
    .with_workspace_dir(app_data_dir)
    .with_clipboard(TauriClipboardWriter)
    .with_notifications(TauriNotificationSink)
    .build();
```

测试时替换为：

```rust
let runtime = TestAppRuntimeBuilder::new()
    .with_fake_generation(FakeNovelAi::success())
    .with_memory_storage()
    .with_memory_keyring()
    .with_null_notifications()
    .build();
```

## 测试边界

目标：任何 feature workflow 都能在不启动 Tauri、不访问真实文件系统、不访问真实 NovelAI、不访问 keyring 的情况下测试。

### feature unit tests

测试纯领域规则：

- Prompt diagnostics。
- preset override 校验。
- Anlas 估算。
- safety manual override 合成。
- job state transition。

### kernel workflow tests

用 fake ports 组合完整 workflow：

- submit work：compile -> enqueue -> fake generation -> fake resource store -> fake gallery index。
- cancel/retry：状态机和事件顺序。
- replay：从 fake artifact manifest 回填请求。
- Vibe ensure encoding：cache miss -> fake NovelAI encode -> store bucket。

### app use case tests

使用 in-memory adapters 或 fake adapters：

- 缺 active key 时 `work_submit` 返回稳定错误。
- secret store 不可用时 API key list/create/update 的错误语义。
- `visual_asset_transfer(clipboard)` 调用 fake `ClipboardWriter`。
- settings patch 返回 restart-required 字段。

### host integration smoke tests

只测试 Tauri command binding、event bridge、capability wiring，不重复业务测试。

## 旧 `nait` 功能映射

| 旧功能面 | 新边界 |
|---|---|
| runtime status/event | `app-api/runtime`、`app/runtime`、`kernel/runtime`、Tauri event bridge |
| settings | `features/settings`、`adapters/database`、`app/settings_use_cases` |
| API key manager | `features/secrets`、`adapters/keyring`、`adapters/database`、`app/api_key_use_cases` |
| subscription/probe | `features/secrets`、`adapters/novelai`、`app/subscription_use_cases` |
| prompt parse/format/functions | `features/prompt`、`app/prompt_use_cases` |
| lexicon catalog/list/search | `features/prompt-lexicon`、lexicon provider、`app/prompt_lexicon_use_cases` |
| chunk/preset/thumb/preview compile | `features/prompt-resources`、`features/resource-catalog`、`app/resource_use_cases` |
| work submit/get/cancel/retry/delete | `features/jobs`、`features/generation`、`features/prompt-resources`、`features/artifacts`、`features/gallery`、`features/safety`、`kernel/workflows/generate` |
| work batch operations | `features/jobs`、`kernel/workflows/batch_ops`、`app/work_use_cases` |
| work prompt detail/replay | `features/prompt-resources`、`features/artifacts`、`kernel/workflows/replay` |
| gallery list/safety update | `features/gallery`、`features/safety`、`adapters/database`、`app/gallery_use_cases` |
| visual asset resolve/transfer | `features/artifacts`、`features/resource-catalog`、`adapters/storage-fs`、`adapters/image-codec`、host adapters |
| director run | `features/director`、`adapters/novelai`、`features/resource-catalog`、`features/gallery`、`features/safety`、`kernel/workflows/director` |
| vibe import/encode/export/list/get/hide/rename/preview | `features/vibe`、`features/resource-catalog`、`adapters/novelai`、`adapters/storage-fs`、`adapters/database`、`kernel/workflows/vibe` |

## 维护检查

新增 use case 前必须回答：

- 它属于哪个 feature？
- 是否需要 kernel workflow？
- 需要哪些 ports？
- 是否引入了新的真实 I/O？
- 是否应先接入 `resource-catalog`？

新增 adapter 依赖前必须回答：

- 该依赖是否只出现在 `adapters/*` 或 desktop host？
- 是否有 fake/in-memory 替代实现？
- 错误是否在 adapter 边界转换为应用错误？
- 外部库类型是否没有泄漏到 feature、kernel、app-api？

新增可持久资源前必须回答：

- `ResourceKind` 是什么？
- owner 是哪个 feature？
- lifecycle 是 workspace-scoped、job-scoped、cache 还是 export-only？
- base metadata 与 feature metadata 如何分界？
- 是否需要 variant、GC、hash 或 mime/dimension 索引？
