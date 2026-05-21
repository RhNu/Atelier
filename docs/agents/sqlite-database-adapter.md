# SQLite database adapter 决策记录

## 状态

- Date: 2026-05-20
- Status: Implemented direction

## 决策

`adapters/database` 使用 SQLite 作为当前真实持久化引擎，并通过 `rusqlite` 的 bundled SQLite 构建。该 adapter 实现当前已经落地 workflow 所需的持久化 ports：

- `ResourceCatalogRepository`
- `GenerationPayloadStore`
- `JobQueueRepository`
- `RunHistoryRepository`
- `VibeRepository`
- `ArtifactRepository`
- `GalleryIndex`
- `SettingsRepository`

Job queue 只持久化当前 active queue snapshot。运行历史单独记录 batch/job 级 summary 和输出资源引用；它不是 append-only event audit log。Workspace 重开时，未完成队列恢复为 paused，不自动继续消耗 Anlas。

## 编码边界

持久化格式由 adapter 内部 DTO 和 `JsonCodec` trait 承担，不给 feature/domain 模型增加面向持久化的 `Serialize` / `Deserialize` 派生。

设计理由：

- feature crate 不把 SQLite 或 JSON schema 当成领域契约。
- schema 迁移和兼容策略集中在 adapter 内部。
- 可查询字段使用关系列保存，例如 resource state、owner link、gallery filter 和排序字段。
- 不需要查询的复杂嵌套 payload 使用 adapter-local JSON DTO，例如 generation payload、artifact assets、gallery item 和 Vibe document summary。

## 当前 schema 范围

v1 migration 创建：

- `schema_migrations`
- `resources`
- `resource_links`
- `resource_variants`
- `orphan_blobs`
- `generation_payloads`
- `vibe_documents`
- `vibe_encodings`
- `artifacts`
- `gallery_items`

v2 migration 创建：

- `api_key_records`
- `prompt_chunks`

v3 migration 创建：

- `workspace_settings`

v4 migration 创建：

- `generation_queue_state`
- `run_history`
- `run_outputs`

`gallery_items` 为 `indexed_at_ms`、`artifact_kind`、`source_kind`、`manual_safety_override` 建立查询索引。`vibe_encodings` 用 `VibeEncodeSettings::cache_key(source)` 作为稳定 cache key。

## 非目标

- database adapter 不直接暴露 Tauri command；命令入口由 `app` command facade 和桌面 host 负责。
- database adapter 不决定 `app-api` DTO；它只维护 adapter-local persistence DTO。
- 不把 database adapter 变成业务规则 owner。
- 不让 SQLite 类型泄漏到 feature、kernel 或 app-api。
