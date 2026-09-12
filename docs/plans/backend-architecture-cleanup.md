# 后端架构整理：计划与实施记录

日期：2026-09-12。检查基线：`470665350cc8323176cb49c9ce157c18159232f2`。
状态：已完成，按 9 个批次提交并通过验收。以下保留基线审查；实际架构以执行记录和 architecture.md 为准。

## 执行记录

| 批次 | 提交 | 结果 |
| --- | --- | --- |
| 1 | `ae44b74` | 单一 RuntimeDependencies；composition/runtime/session 分离；移除 AppInner、组合构造入口和生产 transient registry。 |
| 2 | `d514c82` | 六组用例缩窄依赖；删除 JobRepository、JobEventSink、with_prompt、ports_ref 等空置接口。 |
| 3 | `969472a` | ImageInputResolver 统一图片读取；reference 纯校验进入生产路径；删除旧 reader/service/kernel workflow。 |
| 4 | `40805ca` | 统一提示词准备；新提交保存 JSON v4 编译快照，v3 经 legacy_prompt 保留旧执行语义。 |
| 5 | `9d0d829` | 共享 KernelOutputPorts；按输出事实记录历史；GenerationStore 定义队列/历史原子提交；外部执行后不回滚成可重发状态。 |
| 6 | `6240425` | WorkflowContext 和 QueueView 分离执行依赖与状态；Vibe/Director 不占生成锁；worker 固定 session 并检查替换身份。 |
| 7 | `b5431a8` | 新提交和重放统一投影完整历史后原子提交；锁内复核唯一性；元数据故障回滚队列与历史。 |
| 8 | `fc3cd3b` | 局部化用例/mapping imports；Explore/Danbooru 功能归位；host worker/platform/commands 与下载 lifecycle/legacy 拆分。 |
| 9 | 最终清理提交 | 删除无消费者的 PageQueryDto/PageInfoDto，并重新生成前端类型；保留有实际消费者的 DanbooruMediaVariantDto。 |

关键回归新增了：提示词跨资源编辑/重启冻结、函数形文本不重复解释、旧/新 payload 版本约束、reference 空输入/错误资源类型、终态持久化失败不重复生成、提交元数据故障原子回滚、阻塞网络下状态/Vibe 读取、旧 worker 不推进新工作区。

实施后的边界：生成队列保留串行写入，执行借用覆盖状态变更与持久化；状态读取只占短锁，Vibe/Director 使用独立执行上下文。文件、远端生成和 SQLite 仍是不同的失败边界；重启恢复保持 paused，未承诺远端 exactly-once。

保留的兼容路径均有明确范围：旧 submitted JSON v3、现有数据库/settings 迁移和 pre-0.5 模型目录清理。旧 `apply_prompt_presets` 没选 preset 时不编译，因此旧请求不能一律标记为已编译。新请求采用显式 v4 快照，不靠文本语法猜测阶段。

模块规模变化：desktop.rs 约 789 → 294 行；Tauri commands.rs 779 → 61 行；下载 manager.rs 734 → 558 行；usecases/mod.rs 收敛为 29 行模块注册与导出。line-budget 警告从 13 降为 9，无超限失败；剩余警告包含测试文件，不以机械拆行作为验收标准。

以下发现与实施顺序是原始基线审查记录；其中旧路径与旧结构已按上表迁移。

## 判断与范围

建议保留 feature-first 的 crate 分层，重点重构 `app` 的装配、用例依赖和生成执行链。长命名主要是依赖组合与职责堆积的结果；单独改短名字，下一次功能升级仍会继续追加后缀。

本次检查覆盖 workspace 清单、backend 指导文档、Rust 符号与调用方、应用装配、kernel workflows、队列与历史、资源与输出、adapter 边界、Tauri host，以及相关测试。对候选废弃 DTO 另外检查了类型导出工具与前端引用。这是静态架构审查，不是所有行为缺陷均已复现的正确性证明。

没有发现 feature/kernel 普遍直接执行文件、SQLite、HTTP 或 keyring I/O 的情况。`prompt-resources` 直接取系统时间是可测试性改进项，但不是此次主要矛盾。`features/generation` 对 bridge 模型描述的依赖、Explore 独立协议适配，都已有明确设计依据，应保留。

## 具体发现

| 编号 | 证据与现状 | 建议及优先级 |
| --- | --- | --- |
| A1 | `app.rs`（基线文件） 有 5 个公开的 workspace 打开入口和 1 个内部入口。`open_workspace_with_dependencies_and_extractor_and_safety_scanner` 长 65 字符；[commands/mod.rs](../../crates/app/src/commands/mod.rs) 还有两种长 62 字符的构造入口，4 处重复完整运行时字段初始化。 | **先做**：用显式依赖对象和单一构建入口替代组合式构造函数。必需的持久设置与账户仓库明确注入；可选能力按职责成组。 |
| A2 | 生产启动走 `AtelierRuntime`，直接打开 `WorkspaceSession` 的便利入口仍创建 transient API key registry，并提供 `session.account()`。仓库内这些直接入口的调用集中在 integration tests；全局账户已在提交 `76d9a2e` 升级为 application scope。 | **先做**：统一生产与测试的装配路径。会话只持有对应用级服务的共享依赖；移除会话创建独立账户体系的便利 API。测试用内存仓库放在显式 test support 中。 |
| A3 | `WorkspaceSession` 包裹单一 `AppInner<S,F,E>`；Prompt、Settings、Gallery 等 use case 都持有完整 session，并传递与自身无关的 `S,F,E`。`AppKernelPorts<S,F,E>` 同时负责网络、Vibe 仓库、图片处理、安全扫描、资源归属和事件。 | **高**：按用例所需服务收窄引用，逐步减少泛型传播。先收窄能力，再决定哪些运行时边界使用 `Arc<dyn Port>`，不要把所有领域服务统一改成动态分派。 |
| A4 | [generation.rs](../../crates/app/src/usecases/generation.rs) 的 `apply_prompt_presets` 调用 `compile_generation_prompt`，已经展开主/负面/角色提示词；[kernel workflow](../../crates/kernel/src/workflow/generation.rs) 执行时又调用 `compile_generation_prompts`。两边还有不同的 compiled prompt 结构。 | **高**：建立单一编译入口和明确的输入阶段。重复编译本身已确认；转义语法、trace 丢失和重放差异是待回归验证的风险，不能直接断言所有请求会出错。 |
| A5 | [generation_persistence.rs](../../crates/app/src/usecases/generation_persistence.rs) 使用具体 database 方法 `commit_queue_and_history`；[jobs ports](../../crates/features/jobs/src/ports.rs) 仍只有分开的 snapshot 保存/清除。submit、执行、rerun 还分别补写 history 字段。 | **高**：让队列与历史的原子提交成为真实端口契约。收敛投影与提交路径，保留现有 SQLite 事务。失败回滚须区分纯内存状态变化和已经发生的外部副作用。 |
| A6 | `persist_generation_outputs` 为一个 job 分页查询全部 Generation Gallery 条目，再筛选 job_id 写输出历史；Director 则从执行返回的 item 直接写。 | **高**：执行路径直接产生输出事实，统一驱动 Artifact/Gallery/RunOutput 更新。停止通过 Gallery 全量反查补写 provenance；保留这些对象各自的所有权和删除语义。 |
| A7 | [KernelGenerationPorts 与 KernelDirectorPorts](../../crates/kernel/src/ports.rs) 各自声明 register resource/artifact、score、index 操作；[app ports](../../crates/app/src/ports.rs) 两组实现执行相同逻辑，仅 Director 方法多一个词。 | **中高**：抽取小型输出处理能力/协作对象，复用确实相同的资源、变体、扫描与索引步骤。保留生成 seed/replay 与 Director result 的差异。 |
| A8 | `run_job_cancellable` 持有 session 的 kernel 锁跨越完整执行；Vibe import/export 和 Director 也取得同一把锁。`KernelRuntime<P>` 把队列、事件序号和并不依赖队列的工作流绑定。 | **中高，独立行为阶段**：从 kernel 中分离队列状态与执行能力，先移出不使用队列的工作流。后续以 operation/session identity 防止旧执行结果写回新状态，再缩短生成锁范围。不要直接删锁。 |
| A9 | `JobRepository`、`JobEventSink` 只有定义与 re-export，未找到实现或调用；`GenerationWorkRequest::with_prompt` 未找到调用；`ports` 与 `ports_ref` 返回同一字段；`KeyringSecretStore::native` 注释明确保留永不失败的 Result 仅为 API compatibility。 | **先做候选清单**：删除无消费者的旧端口和 convenience API，收窄可见性；移除无实际失败语义的内部 Result。删除前仍以全 workspace、all-targets 编译确认。 |
| A10 | [precise-reference workflow](../../crates/kernel/src/workflow/precise_reference.rs) 只有 kernel 测试调用；实际 app 用 `character_reference_to_domain`。旧路径先异步读，再包装同步 reader 供 service 读取；app 的旧 port 还先读 base64、再读 bytes 取 kind。 | **中高**：统一实际 reference 输入处理。将有价值的 kind/empty 校验接入生产共享 resolver 后，移除仅为适配同步/异步而存在的包装。是否保留独立 feature crate，以剩余领域职责决定，不能连同校验一起删。 |
| A11 | [usecases/mod.rs](../../crates/app/src/usecases/mod.rs) 与 [mapping.rs](../../crates/app/src/mapping.rs) 汇总大量跨 feature imports，子模块从父模块取依赖。相同 job status 映射、variant 字符串映射和图片输入解析有重复。新 Explore/Danbooru 的应用流程直接放在 commands，其他能力多经 usecases。 | **中**：按功能把 mapping、流程和依赖放在一起；command 只做入口选择和 error envelope。合并 app 内同义转换，但保留数据库 codec、API DTO、bridge wire 的独立边界。 |

判断为应保留或须谨慎处理的部分：

- SQLite 和全局 settings 的 `vN_to_vN+1` migration、adapter-local JSON DTO、严格版本检查是兼容边界，不属于仅为旧抽象存活的转发层。
- `cleanup_legacy_image_analysis` 仍是启动调用的一次性磁盘清理；先隔离到 adapter 的 upgrade 模块。只有明确停止支持其来源版本，才移除执行逻辑及相应状态。
- `DanbooruMediaVariantDto` 虽没有后端运行时消费者，前端 `DanbooruImage.tsx` 仍引用，不能当死类型删除。`PageQueryDto`、`PageInfoDto` 当前只查到声明和导出链，是待清理候选；不借此统一所有分页协议。
- Kernel workflow facade、feature service、adapter trait implementation 不能仅因函数短就删除。判断依据是有没有独立的领域、错误、执行或 I/O 边界。
- Tauri 的 JoinHandle、原生对话框、文件选取、更新检查和通知属于 host 职责。`desktop.rs` 较长说明模块混杂，不意味着所有内容都应移入 kernel。

## 目标结构与取舍

先在现有 crate 内调整，不预先增加 `core`、`protocol`、通用 service registry 或横跨所有功能的 ApplicationService。

| 层 | 调整后的职责 |
| --- | --- |
| Desktop host | 平台能力、进程启动装配、任务 spawn/join/abort、窗口事件与通知。 |
| App runtime | 持有应用级账户、设置、下载/分析/词库/Explore 能力；管理 workspace session 生命周期。构造完成后不再替换核心依赖。 |
| Workspace session | 持有工作区资源、repositories、feature services 与生成运行状态，借用/共享应用级服务。 |
| App use cases | 按功能组织 DTO 映射、guard 与应用流程，只拿所需服务。commands 不另建一套业务逻辑。 |
| Kernel | 无真实 I/O 的跨 feature 编排。生成状态与执行协作分离；Vibe、Director 不借用生成队列锁。 |
| Features / adapters | 保留领域与 I/O 分界；补齐真正需要的原子提交端口，删除空置端口和无用途包装。 |

建议的 API 形状，名称可在实施时按具体职责微调：

```text
AtelierRuntime::new(RuntimeDependencies)
WorkspaceSession::open(root, WorkspaceServices)   // 内部装配入口
GenerationHistory::project(snapshot, existing, now)
GenerationStore::commit(change)                  // 端口归 jobs；SQLite 实现归 adapter
ReferenceResolver::resolve(input)
```

`RuntimeDependencies` 是装配参数，不是可供每个用例随意查找服务的大容器。共享账户、设置必须明确提供；Safety 的 scanner、policy control、session control 要表达来自同一套能力。默认不可用的词库/网络能力可以保留，不能用隐式 transient persistence 掩盖生产配置遗漏。

App 内建议逐功能聚合：`runtime`、`composition`、`session` 管生命周期与装配；`generation`、`prompt`、`history`、`explore` 等模块各自放置用例和 mapping。外部 facade 可保留稳定方法，内部不再通过 `usecases/mod.rs` 的全局 imports 连接各功能。先迁移一个功能验证形状，再逐个收敛，不为这次整理额外拆出一批 crate。

提示词建议继续在提交时冻结已解析输入，符合现有 `generation_submit_applies_prompt_presets_before_queueing_work` 行为。预览、token count、估算和提交复用同一编译服务；执行消费已解析输入，不再解释其文本。原始输入、编译结果、request plan 的区别有领域意义，应以类型/字段表达，不能为少几个结构而全部合并。完整 trace 的持久化是否扩充格式，单独评估。

提交时冻结意味着：排队后修改 preset/chunk 不应改变已提交工作；显式“重新按当前设置生成”与 replay 旧请求也需要区分。现存持久 payload 不能仅靠字符串中是否出现 `$chunk` 等语法来猜测其阶段；须用支持版本的样例核对实际语义，需要格式演进时仍走显式 migration。

输出处理必须承认网络、文件和 SQLite 不是一个事务：按 job/sample/asset identity 实现本地幂等记录，及时保存部分成功结果，并区分请求失败与结果落盘失败。数据库提交失败不能自动把已完成的付费生成视为未执行并重发；这是阶段 4 的失败注入重点，不承诺远端请求的 exactly-once。

内部命名规则：用所属模块/类型承载上下文，方法描述动作，不把依赖列表写进名字。`project`、`resolve`、`commit` 适合窄职责对象；Tauri command 的功能前缀、重要语义后缀和有描述力的测试名可以保留。不设统一的函数名字数硬限制，也不靠缩写降低可读性。

## 实施顺序

| 阶段 | 可独立评审的变更 | 完成条件 |
| --- | --- | --- |
| 0：行为与删除清单 | 记录现有支持格式、IPC/生成类型链、真实调用方；对下列关键行为保留既有测试并补足缺口。 | 每个删除项都有消费者结论；明确哪些是纯结构调整、哪些会影响执行语义。 |
| 1：装配统一 | 引入单一 RuntimeDependencies 构造；移除组合构造函数和测试专用生产捷径；拆出 composition/runtime/session；整理 AppInner。 | 增加可选能力只需新增字段/小能力对象，不再新增依赖组合函数；生产和测试走同一构建路径；账户跨工作区保留。 |
| 2：缩窄依赖与清除残留 | 收窄 Prompt/Settings 等用例依赖，局部化 imports/mapping；清理 A9 无消费者 API；统一 reference resolver；将 Explore/Danbooru 流程按职责归位。 | 无关用例不再携带 S/F/E；旧入口和 re-export 同步移除；reference 校验进入生产路径。类型导出项与前端真实消费者一并核对。 |
| 3：请求准备统一 | 合并预览/token/estimate/submit 的编译组装规则，明确 raw/resolved/planned 阶段；去除 kernel 对已解析请求的重复编译。 | main/negative/character、model binding、preset override、转义语法、retry/replay 都遵循同一语义；历史 payload 可恢复。 |
| 4：输出与提交统一 | 先抽取共享输出能力，再由执行直接记录输出；将 queue/history 原子提交加入 jobs port；合并新提交、重放与状态投影的重复逻辑。 | 每个输出只建立一组事实；无 Gallery 全量反查；终态、输出、tombstone 一致；持久化失败不会被简单内存回滚误认为外部操作未执行。 |
| 5：执行边界调整 | 把无队列依赖的 Vibe/Director workflows 移出生成 runtime 锁；建立执行身份校验，再缩短生成锁；worker 内部使用领域 directive，DTO 留在命令边界。 | 活跃生成不阻塞无关 Vibe/状态读取；暂停/停止/取消规则不混淆；旧 session/旧 run 的完成消息不会推进新队列；shutdown 正确等待并释放 lease。 |
| 6：收尾与防回流 | 按职责拆分 host commands/desktop、download manager 大模块；隔离升级清理；删过渡别名与重复实现；更新长期架构说明。 | 没有未注明退出条件的临时旧路径；新增功能可沿现有边界接入；检查与测试全部通过。 |

阶段 1、2 是首批，主要兑现命名和结构减负；阶段 3、4 是数据流收敛；阶段 5 单独处理并发语义，不能夹带在“纯重命名”提交里。每阶段可以拆成多个小 PR；无需等待所有模块统一改完才合并，但同一职责不长期保留新旧两套入口。

不建议先大规模重命名所有 crate/DTO，也不建议先上完整 actor framework 或泛化 UnitOfWork。先解决已经确认的边界问题，需要新的抽象时，以实际消费者和原子性要求证明它的用途。

## 验证与回归边界

| 变更面 | 必须保持或补充的行为证据 |
| --- | --- |
| 装配、会话 | 无 workspace 可管理账户；跨 workspace 使用同一 registry；打开候选失败不替换当前 session；记录 recent workspace 失败时不发布候选；关闭失败可重试。 |
| Prompt / reference | 模型绑定、主/负面/角色 preset、quality/UC override、escaped function-like 文本；提交后修改资源不改变排队输入；resource/inline 输入、无效 kind、空内容；估算/token 与实际提交一致。 |
| 队列与持久化 | 重启恢复为 paused；submit/rerun 的唯一性；原子 commit 失败；副作用发生后的持久化失败；多 sample、部分成功、stream cancel 与 retry 不重复建立输出。 |
| 输出所有权 | 删除 Gallery 保留历史 tombstone；删除 history 不删 Artifact/Gallery；资源归属和缩略图/preview 的 best-effort 行为保留；安全扫描 unavailable/failed/scanned 和 manual override 不混淆。 |
| 执行生命周期 | 生成期间状态读取及无关工作；同 session 的并发修改；旧 worker finish、pending directive、workspace replacement、shutdown/abort 和 lease 释放；事件序号与作用域在拆分后保持一致。 |
| 格式和 IPC | 所支持数据库/settings 版本、损坏/未知/未来版本拒绝、迁移事务回滚；adapter JSON 映射不受内部 rename 影响；公开 command/error/event/DTO 若变化，同步消费者与生成器。 |

已有可复用测试集中在 `crates/app/tests/command_facade/session_commands.rs`、`app_integration/queue_recovery.rs`、`app_integration/generation_outputs.rs`、`director_safety_history.rs`、kernel workflows、database integration tests 和 desktop worker tests。仅有测试名或 happy path 通过，不等于以上失败注入和并发场景已覆盖。

每个 Rust 实施批次按仓库规则运行 `cargo fmt --all -- --check`、`cargo clippy-strict`、`cargo test --workspace`、`cargo xtask line-budget`。涉及生成的 TS 或前端调用时，再运行 `pnpm fmt:check`、`pnpm lint`、`pnpm test`。按真实行为缺口补测试，不为简单 rename 镜像实现。

所有 Rust 批次均通过 `cargo fmt --all -- --check`、`cargo clippy-strict`、`cargo test --workspace`、`cargo xtask line-budget`。最终类型生成后还通过 `pnpm fmt:check`、`pnpm lint`、`pnpm test`、`pnpm build`。line-budget 保留 9 个超过 600 行的 warning，无超限失败；前端构建保留大 chunk 提示，不影响构建通过。

## 最终验收

- 新增一个可选后端能力不再改变一串构造函数名，也不迫使无关 usecase 修改泛型。
- workspace/session/runtime 各有唯一生命周期职责，测试不依赖生产绕行入口。
- 一次生成只使用一份权威提示词编译结果；执行、输出事实与队列/历史提交的责任明确。
- 无消费者的端口、重复 getter、兼容性 convenience API 和临时别名已移除；仍需保留的升级逻辑有清楚的版本边界。
- 数据库兼容、账户作用域、资源删除、恢复、取消和 host 生命周期行为通过验证；长期架构文档描述最终实现。
