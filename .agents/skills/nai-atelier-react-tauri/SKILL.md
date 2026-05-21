---
name: nai-atelier-react-tauri
description: Use when working on NAI Atelier frontend code in apps/desktop, including Tauri webview UI, Vite React components, TanStack Router or Query flows, Zustand state, forms, rendering, accessibility, desktop host boundaries, or React performance review.
---

# NAI Atelier React Tauri

## Overview

Use this skill for NAI Atelier's local desktop frontend. Adapt React performance guidance to a Tauri v2 webview: optimize interaction latency, rendering, state ownership, and bundle shape without importing web deployment, SSR, or generic AI image platform assumptions.

## First Checks

- Read the project root `AGENTS.md`, `README.md`, and `docs/agents/README.md` before frontend design or implementation work.
- Keep UI language and workflows NovelAI-specific: prompt resources, generation work, jobs, artifacts, gallery items, Vibe documents, Director results, workspace settings, and safety assessments.
- Work in `apps/desktop` for frontend code. Treat `apps/desktop/src-tauri` as desktop host glue, not an application domain layer.
- Prefer the existing stack: React 19, Vite, TanStack Router, TanStack Query, Zustand, React Hook Form, Zod, lucide-react, Vitest, Testing Library, oxlint, and oxfmt.

## Desktop Boundary Rules

- Use Tauri commands for desktop actions that involve native dialogs, selected file reads or writes, reveal/open actions, notifications, bundled resource paths, keyring, database, NovelAI calls, or workspace filesystem access.
- Do not add arbitrary frontend filesystem access for user files. Combine picker, validation, read/write, and host action in the Tauri command or host-neutral app facade.
- Keep NovelAI network details behind backend adapters. The frontend should express product intents and app-api DTOs, not `novelai-bridge` or provider protocol details.
- Use TanStack Query for async frontend state that represents command results, cached app data, or refreshable resources. Use Zustand for local UI state that does not need server-style cache semantics.
- Validate frontend-facing payloads at boundaries with existing DTOs or Zod when runtime validation is needed; avoid duplicating Rust domain rules in React.

## React Guidance

Use the curated rules in `references/react-performance-rules.md` when writing, reviewing, or refactoring UI code. Apply them pragmatically:

- Prevent desktop command waterfalls: start independent Tauri command/query work together, defer awaits until needed, and use cheap synchronous guards before expensive async work.
- Keep renders cheap: derive values during render, subscribe to derived slices, split unrelated hooks, avoid inline component definitions, and memoize only when work is actually expensive.
- Keep input responsive: use `startTransition` or `useDeferredValue` for non-urgent filtering, previews, gallery scans, and generated-result list updates.
- Keep effects narrow: move interaction logic into event handlers, clean up global listeners, use passive scroll/touch listeners where appropriate, and make dependencies primitive when possible.
- Keep desktop bundles understandable: avoid broad barrel imports, keep dynamic imports statically analyzable, and lazy-load heavy optional panels or tools instead of the primary workbench.
- Keep local browser storage small and versioned. Durable workspace data belongs behind backend storage/database/resource-catalog flows, not long-lived `localStorage` blobs.

## Out of Scope

Do not apply guidance for Next.js pages or app router, React Server Components, server actions, API routes, SSR hydration mismatches, `React.cache()`, `after()`, Vercel deployment, CDN/network resource hints, analytics loading, public-web SEO, or generic multi-provider image platforms.

## Verification

For frontend code changes, run and confirm:

```powershell
pnpm fmt:check
pnpm lint
pnpm test
```

Also run `pnpm build` or `pnpm desktop:dev` when the change touches routing, build-time imports, Tauri command wiring, or behavior that only appears in the desktop shell.

## References

- `references/react-performance-rules.md`: curated React rules adapted from Vercel's official React best practices skill.
