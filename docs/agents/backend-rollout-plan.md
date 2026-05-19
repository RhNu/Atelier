# 后端落地顺序计划

## 状态

- Date: 2026-05-19
- Status: Planning direction

本文说明 `backend-crate-layout.md` 里的后端边界应如何逐步落地。它不是代码任务清单，也不要求一次性打通 UI -> Tauri -> NovelAI 的最短路径。目标是让每一步都能独立验证边界、测试替换性和维护规则。

## 原则

- 不做 MVP 半成品。阶段产物可以没有完整用户链路，但必须有清楚边界、测试入口和后续接入位置。
- 先建可替换的 contract，再接真实 I/O。
- 先建统一资源管理，再让 artifact、gallery、vibe、thumb 接入。
- 先写 fake/in-memory adapter 能跑通的 workflow，再接 filesystem、database、keyring、`novelai-bridge`。
- Tauri 接入靠后，只验证 host bridge，不承担业务补洞。
- 每个阶段结束时，必须能说明新增 crate 的 owner、依赖方向、ports、测试替换方式。

## 阶段 0：文档与边界守卫

目标：让后续代码工作有明确路线，不把早期 scaffold 当最终结构。

产物：

- `backend-crate-layout.md` 作为 crate 边界来源。
- 本文作为落地顺序来源。
- 后续可补 `dependency-boundary.md`，记录禁止依赖和推荐检查命令。

验收：

- `AGENTS.md` 和 `docs/agents/README.md` 的阅读顺序包含后端布局与落地计划。
- 旧文档不再暗示“完整 feature crate 清单暂不存在”是长期状态。

## 阶段 1：基础 contract scaffold

目标：只建立薄 crate 和公共 contract，不实现业务链路。

建议创建：

- `crates/app-api`
- `crates/kernel`
- `crates/features/workspace`
- `crates/features/resource-catalog`
- `crates/adapters/*` 的占位 crate 可按需要逐个出现，不一次性创建空目录。

关键内容：

- `foundation` 放最小基础原语和错误骨架。
- `app-api` 放 runtime/error envelope 的最小 contract。
- `kernel` 只放 runtime 状态、event hub 概念和 port 组织位置。
- `workspace` 定义 workspace root、controlled path、layout 规则。
- `resource-catalog` 定义 `ResourceId`、`ResourceKind`、`ResourceRef`、owner、lifecycle。

不做：

- 不接 Tauri command。
- 不接 `novelai-bridge`。
- 不选最终 database。
- 不写 generation 提交链路。

验收：

- workspace 能编译。
- dependency direction 清楚。
- 每个 crate 有最小测试，证明 crate metadata、基础类型或纯函数规则可用。

## 阶段 2：resource-catalog 优先成型

目标：先解决资源统一抽象，避免后续 feature 各自开目录和索引。

关键设计：

- `ResourceKind` 覆盖 generated image、director result、source/reference/controlnet image、prompt thumb、vibe document、vibe preview、vibe encoding。
- `ResourceOwner` 区分 job、gallery、prompt resource、vibe、cache、import staging。
- `ResourceLifecycle` 区分 workspace-scoped、job-scoped、cache、export-only。
- `ResourceMetadata` 只放跨 feature 基础 metadata，例如 mime、size、hash、dimensions、created time。

ports：

- `ResourceCatalogRepository`
- `ResourceBlobStore`
- `ResourceVariantBuilder`

fake 测试：

- in-memory catalog 可以登记资源、查询 resource ref、标记 lifecycle。
- fake blob store 可以验证 controlled path 不泄漏给 feature。
- 删除 owner 时能推导可清理资源，但不需要真实删除文件。

不做：

- 不绑定 redb/sqlite。
- 不规定最终磁盘目录。
- 不把 Gallery 或 Vibe 的领域字段塞进 `ResourceMetadata`。

## 阶段 3：纯 feature crate 逐个落地

目标：优先实现不依赖真实 I/O 的领域规则。

建议顺序：

1. `features/prompt`
2. `features/prompt-lexicon`
3. `features/prompt-resources`
4. `features/safety`
5. `features/generation`
6. `features/jobs`
7. `features/artifacts`
8. `features/gallery`
9. `features/vibe`
10. `features/director`
11. `features/settings`
12. `features/secrets`

顺序理由：

- Prompt 和 prompt resources 是生成链路的输入基础。
- `resource-catalog`、artifacts、gallery、vibe 都依赖统一资源引用。
- generation/jobs 可以先用 fake NovelAI client 验证状态机和 request plan。
- secrets/settings 虽然基础，但真实价值在 app use case 和 adapters 接入时体现，可以在纯模型层后补。

每个 feature 的验收：

- 领域模型归属清楚。
- ports 只描述能力，不泄漏 adapter 类型。
- 测试覆盖核心规则。
- 不能直接出现 filesystem、database、keyring、Tauri、`novelai-bridge` 类型。

## 阶段 4：fake adapters 与 kernel workflows

目标：用 fake/in-memory ports 验证跨 feature 编排，而不是先接真实外部能力。

建议 workflow：

- prompt preview compile：prompt -> prompt-resources。
- generate submit plan：prompt-resources -> generation -> jobs。
- generate completion：jobs -> resource-catalog -> artifacts -> safety -> gallery。
- replay：artifacts -> prompt-resources -> generation request reconstruction。
- vibe ensure encoding：vibe -> resource-catalog -> fake NovelAI encode -> resource-catalog。
- director run：director -> fake NovelAI director -> resource-catalog -> safety -> gallery。

验收：

- workflow tests 不启动 Tauri。
- workflow tests 不访问真实 filesystem。
- workflow tests 不访问真实 NovelAI。
- event 顺序、错误映射、cancel/retry 语义可断言。

不做：

- 不为前端暴露完整 command 面。
- 不把 workflow 临时逻辑写进 `app`。
- 不让 fake adapter 变成生产实现。

## 阶段 5：真实 adapters 分批接入

目标：每次只接一个真实外部能力，并保留 fake 实现。

建议顺序：

1. `adapters/storage-fs`
2. `adapters/database`
3. `adapters/image-codec`
4. `adapters/keyring`
5. `adapters/novelai`
6. `adapters/safety-onnx`
7. `adapters/desktop-system`

顺序理由：

- storage/database 支撑 resource catalog 和 index。
- image codec 支撑 visual asset variants、metadata、preview。
- keyring 与 NovelAI adapter 进入后，subscription、generation、vibe、director 才接真实服务。
- safety ONNX 可以在最终产物链路稳定后接入。
- desktop-system 最后接，避免 Tauri 先行倒逼 app 边界。

验收：

- 每个 adapter 有独立 integration 或 contract test。
- 每个 adapter 有 fake/in-memory 替代。
- 外部库错误在 adapter 边界转换为应用错误。
- 外部库类型不泄漏到 feature、kernel、app-api。

## 阶段 6：app use cases

目标：把 `app-api` DTO 映射到 kernel/feature workflow，形成 host-neutral application runtime。

use case group：

- `RuntimeUseCases`
- `SettingsUseCases`
- `ApiKeyUseCases`
- `SubscriptionUseCases`
- `PromptUseCases`
- `ResourceUseCases`
- `WorkUseCases`
- `GalleryUseCases`
- `VisualAssetUseCases`
- `VibeUseCases`
- `DirectorUseCases`

验收：

- `app` 不依赖 `tauri` crate。
- runtime guard 在 `app` 统一处理。
- API key 预检、错误 envelope、DTO 转换在 app 测试中覆盖。
- `visual_asset_transfer(clipboard)` 通过 trait 调用 host adapter，测试可替换为 fake clipboard。

不做：

- 不把领域规则搬进 use case。
- 不在 use case 里直接读写文件或调用 keyring。
- 不把 command 名作为内部 API 名的唯一来源。

## 阶段 7：Tauri host bridge

目标：只把 desktop host 接到 `app`，不重新实现业务。

内容：

- command wrapper。
- event bridge。
- capability 与 plugin wiring。
- desktop adapters 注册。
- app data dir 注入。

验收：

- command 只做反序列化、调用 `app`、包装响应。
- event bridge 只转发 app/kernel 事件。
- Tauri 不直接依赖 `novelai-bridge`。
- Tauri 不直接访问 database、resource catalog 内部 API 或 feature service。

## 阶段 8：按能力补齐完整功能

目标：在边界稳定后，按完整能力逐个补齐，不用“先能生成一张图”定义完成。

建议能力包：

- Prompt 工作区：parser、formatter、diagnostics、lexicon、completion contract。
- Prompt resources：chunk、preset、thumb、preview compile、trace。
- Generate work：参数、Vibe/reference/controlnet/source image、streaming、batch、retry/cancel。
- Artifact/replay：final image、metadata、resource refs、replay manifest、visual asset variants。
- Gallery：query、filter、manual safety override、跨来源聚合。
- Vibe：import、embedded image import、encode、ensure bucket、export、preview、hide/rename。
- Director：input、tool request、result registration、gallery/safety integration。
- Settings/API key/subscription：multi-key manager、active key、manual probe、secret safety。
- Safety：scan policy、ONNX runtime adapter、degraded warning、manual override。

每个能力包的完成标准：

- feature unit tests。
- kernel workflow tests。
- app use case tests。
- adapter contract tests，如果涉及真实 I/O。
- 文档更新，说明新增 use case、ports、resource kind 或 adapter。

## 反模式

- 为了看到 UI 结果，把业务临时写进 Tauri command。
- 每个 feature 自己创建 `cache/`、`thumbs/`、`gallery/` 文件夹。
- `app-api` 复刻所有内部领域模型。
- `kernel` 直接持有 database 或 keyring client。
- `adapters/novelai` 的 bridge 类型泄漏到 `generation` 或 `app-api`。
- fake adapter 行为与真实 adapter contract 不一致。
- 把“能生成一张图”当成后端架构完成。

## 每次新增 crate 前的检查

- 这个 crate 是否有单一 owner？
- 它依赖哪些 crate？
- 它暴露 domain service、port、adapter 还是 app use case？
- 它是否引入真实 I/O？
- 它的 fake 或 in-memory 替代在哪里？
- 它是否需要接入 `resource-catalog`？
- 它的测试不启动 Tauri 能否验证主要行为？
