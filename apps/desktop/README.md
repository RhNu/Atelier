# NAI Atelier Desktop

NAI Atelier 的 Tauri v2 shell 与 Vite React 前端。

日常开发优先使用仓库根目录脚本：

```powershell
pnpm dev
pnpm desktop:dev
pnpm fmt:check
pnpm build
pnpm lint
pnpm test
```

完成前端工作前，必须确认 `pnpm fmt:check`、`pnpm lint`、`pnpm test` 通过。本 package 使用 Oxc 系列工具：`oxfmt` 负责格式化，`oxlint` 负责 lint。

该 package 应聚焦桌面壳与前端 workbench。除 Tauri-facing glue 外，不要把 Rust 领域代码加入 `src-tauri`。
