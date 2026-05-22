import { Minus, PanelLeftClose, Square, X } from "lucide-react";
import { useCallback, type MouseEvent, type ReactNode } from "react";

import { AppButton, AppIconButton, AppPanel, AppToastHost } from "../components/ui";
import { routeNavItems, type RouteNavItem } from "../routes/nav";
import { useRouteRestoreStore } from "../stores/shell-store";
import type { WorkspaceStatusDto } from "../types";

export type AppShellProps = {
  workspaceStatus: WorkspaceStatusDto | null;
  workspacePending: boolean;
  workspaceErrorCode?: string;
  workspaceErrorMessage?: string;
  activePath?: string;
  children?: ReactNode;
  onOpenWorkspace?: () => void;
  onCloseWorkspace?: () => void;
  onNavigate?: (to: RouteNavItem["to"]) => void;
};

async function controlWindow(action: "close" | "maximize" | "minimize") {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const appWindow = getCurrentWindow();

  if (action === "close") {
    await appWindow.close();
    return;
  }

  if (action === "maximize") {
    await appWindow.toggleMaximize();
    return;
  }

  await appWindow.minimize();
}

function handleMinimizeWindow() {
  void controlWindow("minimize");
}

function handleMaximizeWindow() {
  void controlWindow("maximize");
}

function handleCloseWindow() {
  void controlWindow("close");
}

function getFallbackPath(): string {
  if (typeof window === "undefined") {
    return "/generate";
  }

  return window.location.pathname;
}

export function AppShell({
  workspaceStatus,
  workspacePending,
  workspaceErrorCode,
  workspaceErrorMessage,
  activePath = getFallbackPath(),
  children,
  onOpenWorkspace,
  onCloseWorkspace,
  onNavigate,
}: AppShellProps) {
  const setLastRoute = useRouteRestoreStore((state) => state.setLastRoute);
  const showWorkspaceGate = workspaceStatus === null;
  const fatalBootError =
    workspaceErrorCode !== undefined && workspaceErrorCode !== "workspace_not_open";

  return (
    <div className="flex h-svh min-h-0 flex-col overflow-hidden bg-app-bg text-app-text">
      <header
        data-tauri-drag-region
        className="titlebar-drag flex h-11 shrink-0 items-center justify-between border-b border-app-border bg-app-panel pl-4"
      >
        <div data-tauri-drag-region className="flex items-center gap-3">
          <div className="grid size-6 place-items-center border border-brand-400/50 bg-brand-500/20 text-[11px] font-black text-brand-100">
            A
          </div>
          <div data-tauri-drag-region>
            <p className="text-sm font-semibold text-white">Atelier</p>
            <p className="text-[11px] text-app-muted uppercase">NovelAI workspace</p>
          </div>
        </div>
        <div className="titlebar-no-drag flex h-full items-center">
          <button
            type="button"
            aria-label="Minimize window"
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
            onClick={handleMinimizeWindow}
          >
            <Minus aria-hidden="true" className="size-4" />
          </button>
          <button
            type="button"
            aria-label="Maximize window"
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
            onClick={handleMaximizeWindow}
          >
            <Square aria-hidden="true" className="size-3.5" />
          </button>
          <button
            type="button"
            aria-label="Close window"
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-rose-500 hover:text-white"
            onClick={handleCloseWindow}
          >
            <X aria-hidden="true" className="size-4" />
          </button>
        </div>
      </header>

      {showWorkspaceGate ? (
        <main className="flex min-h-0 flex-1 items-center justify-center p-6">
          <AppPanel className="w-full max-w-xl p-6">
            <p className="text-xs font-semibold text-brand-200 uppercase">Workspace</p>
            <h1 className="mt-2 text-xl font-semibold text-white">Open an Atelier workspace</h1>
            <p className="mt-3 text-sm text-app-muted">
              {fatalBootError
                ? (workspaceErrorMessage ?? "Workspace status could not be loaded.")
                : "Select a workspace folder before running NovelAI generation workflows."}
            </p>
            <div className="mt-5 flex gap-2">
              <AppButton onClick={onOpenWorkspace} disabled={workspacePending}>
                {workspacePending ? "Opening workspace" : "Open workspace"}
              </AppButton>
            </div>
          </AppPanel>
        </main>
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[64px_minmax(0,1fr)]">
          <nav
            aria-label="Workspace sections"
            className="flex min-h-0 flex-col items-center gap-2 border-r border-app-border bg-app-panel px-2 py-3"
          >
            <AppIconButton icon={PanelLeftClose} label="Collapse panels" className="mb-2" />
            {routeNavItems.map((item) => {
              const active = activePath === item.to;
              return (
                <RouteNavLink
                  key={item.to}
                  active={active}
                  item={item}
                  onNavigate={onNavigate}
                  setLastRoute={setLastRoute}
                />
              );
            })}
          </nav>

          <div className="grid min-h-0 grid-rows-[36px_minmax(0,1fr)]">
            <div className="flex min-w-0 items-center justify-between border-b border-app-border bg-app-panel/80 px-3 text-xs text-app-muted">
              <p className="truncate">{workspaceStatus.root}</p>
              <button
                type="button"
                className="text-app-muted hover:text-app-text"
                onClick={onCloseWorkspace}
              >
                Close workspace
              </button>
            </div>
            <main className="min-h-0 overflow-hidden">{children}</main>
          </div>
        </div>
      )}

      <AppToastHost />
    </div>
  );
}

function RouteNavLink({
  active,
  item,
  onNavigate,
  setLastRoute,
}: {
  active: boolean;
  item: RouteNavItem;
  onNavigate?: (to: RouteNavItem["to"]) => void;
  setLastRoute: (route: string) => void;
}) {
  const handleClick = useCallback(
    (event: MouseEvent<HTMLAnchorElement>) => {
      handleNavClick(event, item, setLastRoute, onNavigate);
    },
    [item, onNavigate, setLastRoute],
  );

  return (
    <a
      href={item.to}
      aria-label={item.label}
      aria-current={active ? "page" : undefined}
      className={[
        "grid size-10 place-items-center border transition-colors",
        active
          ? "border-brand-400/70 bg-brand-500/20 text-brand-100"
          : "border-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
      ].join(" ")}
      title={item.label}
      onClick={handleClick}
    >
      <item.icon aria-hidden="true" className="size-5" />
      <span className="sr-only">{item.label}</span>
    </a>
  );
}

function handleNavClick(
  event: MouseEvent<HTMLAnchorElement>,
  item: RouteNavItem,
  setLastRoute: (route: string) => void,
  onNavigate?: (to: RouteNavItem["to"]) => void,
) {
  setLastRoute(item.to);

  if (
    !onNavigate ||
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.altKey ||
    event.ctrlKey ||
    event.shiftKey
  ) {
    return;
  }

  event.preventDefault();
  onNavigate(item.to);
}
