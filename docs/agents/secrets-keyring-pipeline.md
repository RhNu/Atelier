# Secrets 与 Keyring 管线决策记录

## 状态

- Date: 2026-05-20
- Status: Implemented direction

## 决策

`features/secrets` 拥有 API key registry、active key、secret metadata 与显式 subscription probe 规则。真实 secret value 不进入 SQLite、response DTO、event DTO、error details、run history、日志或调试输出；当前只通过 `SecretStore` port 写入系统凭据后端。

`CreateApiKeyRequestDto.secret` 与 `UpdateApiKeyRequestDto.secret` 是唯一允许真实 secret 穿过前端到 Tauri command 边界的位置。它们是 write-only request payload：command 只能把 secret 传给 host-neutral `app` facade 完成创建或更新，不得把 secret 放入响应、事件、持久化记录或调试输出。

`adapters/keyring` 是当前真实 `SecretStore` adapter，service name 固定为 `atelier`，account 使用 `SecretRecordId`。默认测试不触碰 OS keyring；真实 keyring smoke test 保持 `#[ignore]`。

`adapters/database` 在当前 v1 schema 内新增 `api_key_records` 表，只保存：

- `id`
- `display_name`
- `secret_record_id`
- `is_active`

该表不保存明文 secret。active key 切换由 metadata store 保证同一时间最多一个 active key；删除 active key 后不自动选择其它 key。

## NovelAI 管线

`adapters/novelai` 保留显式 key 构造的 `NovelAiBridgeAdapter`，同时新增 resolver-backed adapter。generation、streaming、Vibe、Director 与 subscription 调用前先通过 `SecretResolver` 获取 active key，再构造 bridge client。

Subscription probe 是显式动作：

1. `ApiKeyRegistryService::probe_key` 解析指定 key 的 secret。
2. `SubscriptionProbeClient` 使用该 secret 调用 NovelAI subscription。
3. create/update API key 不访问 NovelAI。

## 非目标

- 本决策初版不新增 `app-api` DTO、host-neutral `app` use case 或 Tauri command；当前实现已经通过 `app-api`、`app` command facade 和 Tauri command 暴露账户管理与显式 probe。
- 前端接入仍不是本文决策范围。
- 不做自动 key 轮换、冷却、失败 fallback 或保存时自动 probe。
