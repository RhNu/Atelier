# Frontend Architecture

## Status

- Date: 2026-05-22
- Status: Current guidance

Atelier desktop is a dense React workbench for NovelAI image workflows. The frontend should feel like a local creative tool: direct, compact, preview-first, and organized around workspace tasks rather than marketing pages or generic provider abstractions.

`D:\Source\_Rust\nait` remains a read-only visual and structural reference. Use it for dark desktop density, thin icon navigation, hard-edged panels, and focused settings/workflow surfaces. Do not copy its source files, assets, command names, DTOs, or Vue-specific patterns.

## Source Layout

The active frontend lives in `apps/desktop/src`.

- `app/`: React Query client and global event bridge.
- `routes/`: TanStack Router setup and top-level workbench layout.
- `shell/`: Tauri webview shell, custom title bar, workspace gate, global navigation, and app-level error handling.
- `platform/atelier/`: the only frontend facade over Tauri commands, event listeners, query keys, and resource image conversion.
- `types/`: generated app-api DTO facade. Feature code imports from `types`, not `types/generated`.
- `components/ui/`: shared, low-level UI primitives with no feature DTO ownership.
- `i18n/`: bundled English and Simplified Chinese resources, typed translation keys, and language preference resolution.
- `features/<feature>/`: feature pages and local feature-owned code.

Feature folders should stay shallow and explicit:

- `data/`: TanStack Query hooks, mutations, and cache invalidation helpers for command-backed data.
- `components/`: feature-specific presentational components when a page grows past a small surface.
- `state/` or `stores/`: local draft, selection, and UI state only when Zustand or local state is not enough in the page.

Avoid recreating a broad horizontal `core`, `protocol`, or frontend `ApplicationService`. If a concept belongs to a NovelAI workflow, keep it in the closest feature folder.

## Data and State

`crates/app-api` is the source of DTO truth. Regenerate TypeScript contracts with `cargo xtask app-api types` only when Rust DTOs change.

Frontend data rules:

- Feature pages do not import `@tauri-apps/api` directly.
- Feature pages do not import `src/types/generated/*` directly.
- `platform/atelier/client.ts` names product intents such as workspace, account, prompt, generation, history, gallery, settings, Vibe, and Director.
- TanStack Query owns async command results, refreshable lists, mutations, and event-invalidated data.
- Zustand is reserved for local UI state such as shell preferences, draft prompts, panel collapse, route restore, and selected items.
- Durable workspace data belongs behind app commands and backend storage, not long-lived browser storage.

Desktop host boundaries:

- Native dialogs, keyring, file picker reads/writes, reveal/open actions, notifications, database, NovelAI calls, bundled resource paths, and workspace filesystem access go through Tauri commands.
- Frontend code may transform DTOs for display, but it should not duplicate Rust domain validation beyond cheap input guards.
- NovelAI protocol details and `novelai-bridge` types stay behind backend adapters.

## Events and Cache

`AtelierEventBridge` listens for `atelier-event` and applies cache invalidation through `platform/atelier/events.ts`.

Cache guidance:

- Query keys are grouped by product domain in `platform/atelier/query-keys.ts`.
- Query keys use explicit `app` and `workspace` prefixes. Workspace-scoped query cache must be cleared when the active workspace changes or closes, while bootstrap and global settings remain cached.
- Event invalidation should target domain roots such as generation, history, gallery, resource, account, or settings.
- Mutations should either update the exact query data returned by the command or invalidate the smallest useful domain.

## Visual System

Atelier uses a dark desktop workbench style:

- Thin global icon rail for primary routes.
- Hard-edged panels and controls with borders, restrained shadows, and compact spacing.
- Dense information layout for repeated creative work.
- Toolbars, tabs, segmented controls, icon buttons, select menus, checkboxes, and numeric inputs for clear controls.
- No rounded corners, decorative orbs, marketing-style landing pages, or single-page catch-all settings surfaces.
- No nested card stacks. Use full-height split layouts, panel columns, or section navigation instead.

Settings pages should use a second-level section navigator inside the Settings route. The global icon rail remains the top-level app navigation; the Settings inner navigator owns Account, Generation, Images, and future Frontend preferences.

### Modal Surfaces

- Standard dialogs use the shared `AppModal`; feature pages should not create their own fixed overlays.
- Modal overlays are portaled to `document.body`, cover the application viewport, and center the dialog on both axes with compact viewport padding.
- Native `dialog` positioning, margin, and padding must be reset so browser defaults cannot move the surface away from the shared layout.
- Dialog height is bounded by the viewport. The title bar stays visible while the content area owns vertical scrolling.
- Every modal has a visible title, an explicit close action, Escape handling, focus containment, and focus restoration.
- Modal content may use feature-owned controls, but backdrop, stacking, placement, and scroll behavior remain responsibilities of `AppModal`.

## Settings Scope

The current Settings frontend separates application and workspace scopes:

- Application / Interface: global language, Gallery SFW blur, and developer-mode preferences stored in the user configuration directory.

## Localization

- Atelier bundles `en` and `zh-CN` resources with `i18next` and `react-i18next`; translations never require network access.
- English is the fallback resource. The `system` preference maps any Chinese OS locale to `zh-CN` and other locales to `en`.
- Product names, Anlas, user content, resource metadata, and backend error details remain untranslated.
- Raw NovelAI model and sampler identifiers remain wire values only. User-facing controls use centralized canonical display names such as `NAI Diffusion 4.5 Full` and `Euler A`; these canonical names are shared across locales.
- User-visible frontend labels, placeholders, actions, empty states, accessibility text, and frontend-owned fallback messages use typed translation keys.
- Workspace: current workspace path and lifecycle action.
- Account: API key registry, active key, and automatically refreshed subscription summary.
- Generation: workspace-local NovelAI image defaults.
- Images: resource thumbnail and preview long-edge sizes.

Do not add runtime/network settings or artifacts root settings without a separate app-api/backend design note.

## Interaction Feedback

- Do not append temporary command results or error lines to an existing form, card, or list item after an interaction.
- Use toast notifications for transient success and non-field errors. Toasts may be persistent and expose one explicit action when confirmation, retry, or navigation is required.
- Interactive controls own their pending state through disabled behavior, progress icons, and temporary action labels.
- Results with an established display surface update that surface in place. Reserve stable placeholders before data is available so loading and refreshes do not shift the layout.
- Keep field validation beside the affected field in a reserved validation area; do not replace actionable field feedback with a global toast.

## Testing

Frontend changes should use focused Vitest and Testing Library coverage:

- Architecture guards for source boundaries and visual-system constraints.
- Feature page tests for user-visible behavior and command payloads.
- Data hook tests when cache invalidation or query-key behavior is non-trivial.
- UI primitive tests for shared controls.

Before completing frontend work, run:

```powershell
pnpm fmt:check
pnpm lint
pnpm test
```

Also run `pnpm build` when routing, build-time imports, Tauri command wiring, or shell-only behavior changes.
