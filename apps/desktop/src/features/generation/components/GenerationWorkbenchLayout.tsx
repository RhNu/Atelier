/* eslint-disable jsx-a11y/prefer-tag-over-role, react-perf/jsx-no-new-object-as-prop */
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
  type ReactNode,
} from "react";

const SIDEBAR_STORAGE_KEY = "atelier.ui.generate.sidebar-width.v1";
const DEFAULT_SIDEBAR_WIDTH = 420;
const MIN_SIDEBAR_WIDTH = 360;
const MAX_SIDEBAR_WIDTH = 520;
const MIN_PREVIEW_WIDTH = 320;
const HISTORY_WIDTH = 280;
const RESIZER_WIDTH = 8;

export function GenerationWorkbenchLayout({
  sidebar,
  preview,
  history,
}: {
  sidebar: ReactNode;
  preview: ReactNode;
  history: ReactNode;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const pointerStartRef = useRef<{ x: number; width: number } | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [sidebarWidth, setSidebarWidth] = useState(readStoredSidebarWidth);
  const maximumWidth = calculateMaximumWidth(containerWidth);
  const clampedWidth = clampSidebarWidth(sidebarWidth, maximumWidth);
  const sidebarWidthRef = useRef(clampedWidth);
  const maximumWidthRef = useRef(maximumWidth);
  sidebarWidthRef.current = clampedWidth;
  maximumWidthRef.current = maximumWidth;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }
    if (typeof ResizeObserver === "undefined") {
      setContainerWidth(container.getBoundingClientRect().width);
      return;
    }
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width) {
        setContainerWidth(width);
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (clampedWidth !== sidebarWidth) {
      setSidebarWidth(clampedWidth);
    }
  }, [clampedWidth, sidebarWidth]);

  const persistWidth = useCallback((width: number) => {
    try {
      window.localStorage.setItem(SIDEBAR_STORAGE_KEY, String(Math.round(width)));
    } catch {
      // The width preference is intentionally best-effort.
    }
  }, []);

  const handlePointerMove = useCallback((event: globalThis.PointerEvent) => {
    const start = pointerStartRef.current;
    if (!start) {
      return;
    }
    const width = clampSidebarWidth(start.width + event.clientX - start.x, maximumWidthRef.current);
    sidebarWidthRef.current = width;
    setSidebarWidth(width);
  }, []);

  const stopPointerResize = useCallback(() => {
    pointerStartRef.current = null;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", stopPointerResize);
    persistWidth(sidebarWidthRef.current);
  }, [handlePointerMove, persistWidth]);

  const handlePointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      pointerStartRef.current = { x: event.clientX, width: clampedWidth };
      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", stopPointerResize);
    },
    [clampedWidth, handlePointerMove, stopPointerResize],
  );

  useEffect(() => {
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopPointerResize);
    };
  }, [handlePointerMove, stopPointerResize]);

  const handleResizeKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      let next = clampedWidth;
      if (event.key === "ArrowLeft") {
        next -= 16;
      } else if (event.key === "ArrowRight") {
        next += 16;
      } else if (event.key === "Home") {
        next = MIN_SIDEBAR_WIDTH;
      } else if (event.key === "End") {
        next = maximumWidth;
      } else {
        return;
      }
      event.preventDefault();
      const width = clampSidebarWidth(next, maximumWidth);
      setSidebarWidth(width);
      persistWidth(width);
    },
    [clampedWidth, maximumWidth, persistWidth],
  );

  return (
    <div ref={containerRef} className="flex h-full min-h-0 min-w-0 overflow-hidden">
      <aside
        data-testid="generation-settings-sidebar"
        className="flex min-h-0 shrink-0 flex-col border-r border-app-border bg-app-panel"
        style={{ width: clampedWidth }}
      >
        {sidebar}
      </aside>
      <div
        role="separator"
        aria-label="Resize generation settings"
        aria-orientation="vertical"
        aria-valuemin={MIN_SIDEBAR_WIDTH}
        aria-valuemax={Math.round(maximumWidth)}
        aria-valuenow={Math.round(clampedWidth)}
        tabIndex={0}
        className="relative m-0 w-2 shrink-0 cursor-col-resize border-0 bg-app-bg outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-app-border hover:after:bg-brand-400 focus:bg-brand-500/20"
        onPointerDown={handlePointerDown}
        onKeyDown={handleResizeKeyDown}
      />
      <div className="min-h-0 min-w-0 flex-1 p-3 pr-0">{preview}</div>
      <div className="w-[280px] shrink-0 p-3">{history}</div>
    </div>
  );
}

function calculateMaximumWidth(containerWidth: number): number {
  if (containerWidth <= 0) {
    return MAX_SIDEBAR_WIDTH;
  }
  return Math.max(
    MIN_SIDEBAR_WIDTH,
    Math.min(MAX_SIDEBAR_WIDTH, containerWidth - HISTORY_WIDTH - MIN_PREVIEW_WIDTH - RESIZER_WIDTH),
  );
}

function clampSidebarWidth(value: number, maximum: number): number {
  return Math.min(maximum, Math.max(MIN_SIDEBAR_WIDTH, value));
}

function readStoredSidebarWidth(): number {
  try {
    const storedValue = window.localStorage.getItem(SIDEBAR_STORAGE_KEY);
    if (storedValue === null) {
      return DEFAULT_SIDEBAR_WIDTH;
    }
    const value = Number(storedValue);
    return Number.isFinite(value) ? value : DEFAULT_SIDEBAR_WIDTH;
  } catch {
    return DEFAULT_SIDEBAR_WIDTH;
  }
}
