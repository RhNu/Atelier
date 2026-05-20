# Kernel generation workflow 决策记录

## 状态

- Date: 2026-05-20
- Status: Implemented direction

本文记录 `nai-atelier-kernel` 首轮 generation workflow 的边界。该实现不是 MVP 直通链路，也不把前端、Tauri command、数据库或真实文件系统纳入本阶段。

## 目标

`kernel` 负责把已有 feature crate 收束成可测试的运行时编排：

- `jobs` 仍拥有 queue、retry、pause、resume、stop 状态机。
- `prompt-resources` 仍拥有 Prompt function compile 与 trace。
- `generation` 仍拥有 NovelAI 参数归一化、request plan 与 Anlas 估算。
- `resource-catalog` 仍拥有资源登记与 lifecycle。
- `artifacts` 仍拥有 artifact 语义与 replay manifest。
- `safety` 仍拥有安全评分模型。
- `gallery` 仍拥有 artifact-backed item 索引规则。

`kernel` 只决定这些能力的执行顺序、事件发射和失败后 queue directive。

## 核心边界

`KernelRuntime<P>` 不启动后台线程。上层显式调用：

- `submit_generation_work`
- `run_scheduled_generation_job`
- `pause`
- `resume`
- `stop`
- `delay_elapsed`

`submit_generation_work` 只保存 submitted payload 并入队，不编译 Prompt、不调用 NovelAI、不登记资源。

`run_scheduled_generation_job` 执行单个已调度 job：

1. `JobQueue::mark_preparing`
2. compile Prompt resources
3. build generation request plan
4. save prepared payload
5. `JobQueue::mark_running`
6. 调用普通生成或 streaming 生成
7. 持久化 final sample、artifact、safety、gallery
8. `JobQueue::mark_succeeded` 或 `JobQueue::mark_failed`

## Streaming 语义

Streaming 是首轮正式路径。`kernel` 对 NovelAI stream chunk 保守处理：

- 每个 `ImageStreamEvent` 都作为 `GenerationStreamChunk` 事件原样发出。
- `event_type` 是 opaque string，`kernel` 不依赖具体字符串判断 final。
- 中间 chunk 不登记到 `resource-catalog`。
- 每个 `sample_index` 只保留最后一个非空 `image`。
- stream 正常结束后，最后一帧 base64 payload 登记为 `ResourceKind::StreamFinalImage`。
- stream 正常结束但没有任何可持久化 image 时，当前 job 失败为 `MissingGeneratedImage`。

## 失败策略

- NovelAI rate limit 使用 `JobFailureImpact::from_novelai_error`，保留 retry delay。
- NovelAI auth、credit、transport、server 等全局错误沿用 `jobs` 的 pause/retry-current 语义。
- safety scanner 失败不会让 job 失败；`kernel` 发 `SafetyScanFailed`，gallery 以 `None` safety assessment 入库。
- resource、artifact、gallery 持久化失败会让当前 job 失败，并返回原始 `KernelError`。
- 已经成功登记的 sample 不在本阶段回滚；后续由 resource repair 和更高层 use case 决定清理策略。

## 非目标

- 不暴露 `app-api` DTO。
- 不确定 Tauri command 名。
- 不选择数据库或持久化 schema。
- 不在 `kernel` 内读取文件、keyring、HTTP、clipboard 或 notification。
- 不把 `novelai-bridge` 类型泄漏进 `kernel`。
