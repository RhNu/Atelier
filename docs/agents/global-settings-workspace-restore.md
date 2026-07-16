# Global Settings and Workspace Restore

Date: 2026-07-15

Atelier separates process-level settings from workspace-local configuration.

- `GlobalSettings` is stored as `global-settings.json` below Tauri's `app_config_dir`. It owns the last successfully opened workspace path and application-wide frontend preferences.
- Global frontend preferences include language, Gallery sensitive-image blur, and developer mode. Language supports `system`, `en`, and `zh-CN`, defaulting to `system`; any Chinese system locale resolves to Simplified Chinese and all others to English. Developer mode defaults to disabled and gates internal resource identifiers and diagnostic metadata. Older schema-v1 files without either field load compatible defaults without a schema migration.
- `WorkspaceSettings` remains in the workspace SQLite database and owns NovelAI generation defaults and image variant sizes.
- `AtelierRuntime` is process-scoped and owns global settings plus an optional `WorkspaceSession`. `WorkspaceSession` owns only one workspace's lock, database, services, and kernel runtime.
- `locks/workspace.lock` is a persistent, empty lock anchor, not a metadata or status file. The storage adapter holds an operating-system exclusive lock on its open file handle, so process crashes release ownership automatically even when the path remains. Explicit workspace close releases the lease before removing the runtime session; desktop exit waits for worker shutdown and that explicit close before terminating the event loop.
- Startup calls `bootstrap_app`. A remembered workspace is reopened automatically. Missing, invalid, or locked workspaces produce a non-fatal restore failure containing the path and error; the remembered path remains until another workspace opens successfully.
- Manual workspace open persists the recent path before publishing the candidate session. Close ends only the current session and does not forget the workspace.
- Frontend query keys are explicitly prefixed with `app` or `workspace`, so workspace switching cannot evict global settings or bootstrap state.

No migration is provided for the former workspace-local frontend preference shape.
