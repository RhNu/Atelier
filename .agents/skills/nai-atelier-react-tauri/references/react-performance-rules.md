# React Performance Rules for NAI Atelier

This reference adapts Vercel React performance rules to NAI Atelier's Tauri v2 + Vite + React 19 desktop webview. Use these rules when implementation details matter; keep `SKILL.md` as the short entry point.

## Async and Desktop Command Flow

- Check cheap local conditions before expensive async work. Do not call a Tauri command or query when a local selection, workspace state, or form state already proves the action is invalid.
- Start independent command/query promises together and await them together with `Promise.all`.
- For partially dependent async work, start each branch as soon as its own prerequisites are ready; do not serialize unrelated Tauri commands behind the slowest prerequisite.
- Defer awaits into the branch that needs the result. Do not block initial UI state on optional panels, metadata probes, or preview-only data.
- Model refreshable command results with TanStack Query. Prefer query keys that describe app concepts and stable identifiers.
- Keep mutation side effects explicit. Invalidate or update only the affected query keys after a Tauri command succeeds.

## Bundle and Import Shape

- Avoid broad barrel imports when they pull large modules into the desktop bundle. Prefer direct imports from the package or local module that owns the symbol.
- Use dynamic imports for heavy optional views such as advanced editors, metadata inspectors, visual analysis tools, or rarely opened settings panes.
- Keep dynamic imports statically analyzable, for example `import("./HeavyPanel")`, not computed paths.
- Load modules only when a feature is activated if the feature is optional in normal desktop use.
- Preload a heavy optional panel on hover, focus, or visible intent when that improves perceived latency without slowing the first desktop window.
- Do not lazy-load the primary workbench shell, core prompt editor, or first-screen controls when that would make the app feel slower.
- Prefer `lucide-react` icons over custom inline SVG for common UI actions; keep custom SVG precision reasonable when SVG is truly needed.

## State and Render Rules

- Derive simple values during render instead of mirroring them through `useEffect` and extra state.
- Subscribe to the smallest useful Zustand slice. Prefer derived booleans or ids over broad objects when a component only needs a narrow fact.
- Use functional `setState` updates to keep callbacks stable and avoid stale closures.
- Use lazy `useState(() => expensiveInitialValue)` for expensive initial state.
- Avoid defining components inside components; it resets identity and state on every render.
- Hoist static JSX outside components when it does not depend on props, state, context, or locale.
- Hoist non-primitive default props for memoized components, for example empty arrays or option objects.
- Use `useMemo` for expensive derived values, not for trivial primitive expressions.
- Split hooks or computations when unrelated dependencies cause unnecessary recalculation.
- Put interaction logic in event handlers. Avoid effects that only react to a click, submit, key command, or menu choice.
- Use refs for transient high-frequency values that do not need to re-render, such as pointer positions, drag state internals, or in-flight keyboard tracking.
- Store latest event handlers in refs or a local `useLatest` helper when a stable external subscription needs current callback behavior.
- Initialize app-wide browser-side services once per app load, not per component mount.
- If using React effect events, do not put effect-event callbacks in dependency arrays.
- Prefer explicit ternaries over `condition && value` when `0`, empty strings, or other falsey values could leak into rendered output.

## Responsiveness

- Use `startTransition` for non-urgent updates that may re-render large UI regions, such as gallery filters, job list regrouping, prompt resource search, or settings previews.
- Use `useDeferredValue` when input should stay responsive while expensive derived rendering catches up.
- Use Suspense boundaries around lazy optional panels or route chunks so loading states stay local. Do not treat this as SSR streaming guidance.
- Use list virtualization or `content-visibility` for long galleries, logs, result grids, and resource lists. Keep keyboard navigation and selection behavior intact.
- Animate a wrapper around SVG content instead of animating complex SVG internals when CSS transforms can do the work.
- Batch DOM style changes through classes or CSS variables instead of repeated layout reads and writes.
- Use passive listeners for wheel, touch, and scroll events when the handler does not call `preventDefault`.
- Clean up global event listeners and avoid one listener per component instance when one shared listener or scoped owner is enough.
- Use React's state-preserving hide/show primitives only if they are available and already accepted by the project stack; otherwise keep visibility state explicit.

## Local Storage and Browser APIs

- Keep `localStorage` small, versioned, and limited to UI preferences that are safe to lose or migrate.
- Cache repeated storage reads in component or module state instead of reading storage repeatedly in hot paths.
- Put workspace data, artifacts, prompt resources, generated metadata, and secrets behind backend storage, database, keyring, or resource-catalog flows.
- Use `requestIdleCallback` only for non-critical browser-side cleanup or warmup. Do not rely on it for required persistence or command completion.

## JavaScript Data Work

- Build `Map` or `Set` indexes for repeated lookups across jobs, gallery items, resources, or prompt tokens.
- Cache repeated pure function results in a bounded module-level `Map` only for deterministic, small, non-durable values.
- Cache repeated property reads inside tight loops when profiling or obvious hot paths justify it.
- Combine multiple array passes in hot paths when readability remains acceptable; use `flatMap` for map-and-filter work when it stays clear.
- Use early exits and cheap length checks before expensive comparisons.
- Hoist regular expressions used repeatedly in tokenization, filtering, or highlighting.
- Use immutable array helpers such as `toSorted()` when available instead of mutating data that React state or caches still reference.
- Use a loop for min/max extraction in hot paths instead of sorting just to read one value.

## Desktop-Specific Review Questions

- Is this UI asking the backend for product-level intent, or leaking filesystem, NovelAI protocol, or adapter details into React?
- Can two independent Tauri queries start together?
- Will this component re-render on every keystroke, pointer move, job event, or gallery update?
- Does this effect represent synchronization with the outside world, or should it be render-time derivation or event-handler logic?
- Is this data durable workspace state? If yes, it should not live only in browser storage.
- Does the import pattern keep the first desktop window fast without hiding core controls behind unnecessary lazy boundaries?

## Explicitly Removed Upstream Rules

The upstream Vercel skill includes guidance for Next.js API routes, server actions, React Server Components, `React.cache()`, `after()`, SSR hydration mismatches, Vercel resource hints, analytics loading, and public web network optimization. Those rules do not apply to this local desktop webview unless a future architecture decision introduces the relevant runtime.
