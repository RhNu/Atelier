# Global Settings and Workspace Restore

Date: 2026-07-15

Atelier separates process-level settings from workspace-local configuration.

- `GlobalSettings` is stored as `global-settings.json` below Tauri's `app_config_dir`. It owns the last successfully opened workspace path and application-wide frontend preferences.
- `WorkspaceSettings` remains in the workspace SQLite database and owns NovelAI generation defaults and image variant sizes.
- `AtelierRuntime` is process-scoped and owns global settings plus an optional `WorkspaceSession`. `WorkspaceSession` owns only one workspace's lock, database, services, and kernel runtime.
- Startup calls `bootstrap_app`. A remembered workspace is reopened automatically. Missing, invalid, or locked workspaces produce a non-fatal restore failure containing the path and error; the remembered path remains until another workspace opens successfully.
- Manual workspace open persists the recent path before publishing the candidate session. Close ends only the current session and does not forget the workspace.
- Frontend query keys are explicitly prefixed with `app` or `workspace`, so workspace switching cannot evict global settings or bootstrap state.

No migration is provided for the former workspace-local frontend preference shape.
