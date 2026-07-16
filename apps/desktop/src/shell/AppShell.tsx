import { Minus, Square, X } from "lucide-react";
import { useCallback, type MouseEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, AppToastHost, LanguageSelect } from "../components/ui";
import { routeNavItems, type RouteNavItem } from "../routes/nav";
import type { FrontendLanguageDto, WorkspaceRestoreFailureDto, WorkspaceStatusDto } from "../types";

export type AppShellProps = {
  workspaceStatus: WorkspaceStatusDto | null;
  workspacePending: boolean;
  workspaceErrorCode?: string;
  workspaceErrorMessage?: string;
  restoreFailure?: WorkspaceRestoreFailureDto | null;
  activePath?: string;
  children?: ReactNode;
  onOpenWorkspace?: () => void;
  onRetryWorkspaceRestore?: () => void;
  onNavigate?: (to: RouteNavItem["to"]) => void;
  language?: FrontendLanguageDto;
  languagePending?: boolean;
  languageErrorMessage?: string;
  onChangeLanguage?: (language: FrontendLanguageDto) => void;
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
  restoreFailure,
  activePath = getFallbackPath(),
  children,
  onOpenWorkspace,
  onRetryWorkspaceRestore,
  onNavigate,
  language = "system",
  languagePending,
  languageErrorMessage,
  onChangeLanguage,
}: AppShellProps) {
  const { t } = useTranslation("shell");
  const { t: translateCommon } = useTranslation("common");
  const showWorkspaceGate = workspaceStatus === null;
  const fatalBootError = workspaceErrorCode !== undefined;

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
          </div>
        </div>
        <div className="titlebar-no-drag flex h-full items-center">
          <button
            type="button"
            aria-label={t("minimizeWindow")}
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
            onClick={handleMinimizeWindow}
          >
            <Minus aria-hidden="true" className="size-4" />
          </button>
          <button
            type="button"
            aria-label={t("maximizeWindow")}
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
            onClick={handleMaximizeWindow}
          >
            <Square aria-hidden="true" className="size-3.5" />
          </button>
          <button
            type="button"
            aria-label={t("closeWindow")}
            className="grid h-full w-11 place-items-center text-app-muted hover:bg-rose-500 hover:text-white"
            onClick={handleCloseWindow}
          >
            <X aria-hidden="true" className="size-4" />
          </button>
        </div>
      </header>

      {showWorkspaceGate ? (
        <main className="flex min-h-0 flex-1 items-center justify-center p-6">
          <AppPanel className="w-full max-w-xl p-6 shadow-app-panel">
            <p className="text-xs font-semibold text-brand-200 uppercase">{t("workspace")}</p>
            <h1 className="mt-2 text-xl font-semibold text-white">
              {restoreFailure ? t("reopenFailed") : t("openTitle")}
            </h1>
            <p className="mt-3 text-sm text-app-muted">
              {restoreFailure
                ? restoreFailure.error.message
                : fatalBootError
                  ? (workspaceErrorMessage ?? t("statusFailed"))
                  : t("selectWorkspace")}
            </p>
            {restoreFailure ? (
              <p className="mt-3 border border-app-border bg-app-surface px-3 py-2 text-xs break-all text-app-muted">
                {restoreFailure.root}
              </p>
            ) : null}
            <div className="mt-5 flex gap-2">
              {restoreFailure ? (
                <AppButton onClick={onRetryWorkspaceRestore} disabled={workspacePending}>
                  {workspacePending ? t("retryingWorkspace") : translateCommon("retry")}
                </AppButton>
              ) : null}
              <AppButton onClick={onOpenWorkspace} disabled={workspacePending}>
                {workspacePending
                  ? t("openingWorkspace")
                  : restoreFailure
                    ? t("chooseAnotherWorkspace")
                    : t("openWorkspace")}
              </AppButton>
            </div>
            {onChangeLanguage ? (
              <div className="mt-5 border-t border-app-border pt-4">
                <LanguageSelect
                  value={language}
                  disabled={languagePending}
                  onChange={onChangeLanguage}
                />
                {languageErrorMessage ? (
                  <p className="mt-2 text-xs text-rose-200">{languageErrorMessage}</p>
                ) : null}
              </div>
            ) : null}
          </AppPanel>
        </main>
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[64px_minmax(0,1fr)]">
          <nav
            aria-label={t("workspaceSections")}
            className="flex min-h-0 flex-col items-center gap-2 border-r border-app-border bg-app-panel px-2 py-3"
          >
            {routeNavItems.map((item) => {
              const active = activePath === item.to;
              return (
                <RouteNavLink key={item.to} active={active} item={item} onNavigate={onNavigate} />
              );
            })}
          </nav>

          <main className="min-h-0 min-w-0 overflow-hidden">{children}</main>
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
}: {
  active: boolean;
  item: RouteNavItem;
  onNavigate?: (to: RouteNavItem["to"]) => void;
}) {
  const { t } = useTranslation("shell");
  const label = t(item.labelKey);
  const handleClick = useCallback(
    (event: MouseEvent<HTMLAnchorElement>) => {
      handleNavClick(event, item, onNavigate);
    },
    [item, onNavigate],
  );

  return (
    <a
      href={item.to}
      aria-label={label}
      aria-current={active ? "page" : undefined}
      className={[
        "grid size-10 place-items-center border transition-colors",
        active
          ? "border-brand-400/70 bg-brand-500/20 text-brand-100"
          : "border-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
      ].join(" ")}
      title={label}
      onClick={handleClick}
    >
      <item.icon aria-hidden="true" className="size-5" />
      <span className="sr-only">{label}</span>
    </a>
  );
}

function handleNavClick(
  event: MouseEvent<HTMLAnchorElement>,
  item: RouteNavItem,
  onNavigate?: (to: RouteNavItem["to"]) => void,
) {
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
