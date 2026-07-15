# Atelier Desktop

This package contains the Tauri v2 shell and Vite React frontend for Atelier.

Prefer root workspace scripts for daily development:

```powershell
pnpm dev
pnpm desktop:dev
pnpm fmt:check
pnpm imports:alias
pnpm imports:alias:check
pnpm build
pnpm lint
pnpm test
```

Before completing frontend work, confirm `pnpm fmt:check`, `pnpm lint`, and `pnpm test` pass. This package uses the Oxc toolchain: `oxfmt` for formatting and `oxlint` for linting.

Use `@/` for source-root imports that would otherwise climb two or more directories. `pnpm imports:alias` rewrites supported module specifiers in handwritten source files and skips generated directories or files marked as generated; `pnpm lint` runs the corresponding check mode.

Keep this package focused on the desktop host and frontend workbench. Do not add Rust domain behavior to `src-tauri` except for Tauri-facing glue.
