# Frontend Workbench Foundation

Date: 2026-05-22

Atelier desktop uses a dense dark React workbench for NovelAI image workflows. The frontend foundation owns the routed shell, local UI state, generated TypeScript DTO facade, Tauri command client, query/event invalidation layer, shared primitives, and the top-level workflow pages.

`D:\Source\_Rust\nait` is a read-only product and visual reference for the dark desktop workbench, thin icon navigation, panel layout, custom title bar, and preview-first generation surface. No `nait` source code or assets were copied into Atelier.

Key choices:

- Tailwind is wired through `tailwindcss` and `@tailwindcss/vite`, with Atelier-owned app and brand tokens.
- `crates/app-api` is the source of frontend DTO shapes. `cargo xtask app-api types` clears and regenerates `apps/desktop/src/types/generated`.
- Frontend Tauri access is isolated under `apps/desktop/src/platform/atelier`; feature pages consume domain clients and TanStack Query hooks.
- Zustand is reserved for local UI state: shell preferences, route restore, draft prompt/editor state, panel collapse, and selected items.
