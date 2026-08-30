import { Outlet, useLocation, useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import { reportBackgroundPromise } from "../app/logger";
import { ActiveAccountRuntime } from "../features/account/runtime/ActiveAccountRuntime";
import { AppUpdateRuntime } from "../features/settings/components/AppUpdateRuntime";
import { ResourceOnboarding } from "../features/settings/components/ResourceOnboarding";
import { useWorkspaceStatus } from "../features/workspace/useWorkspaceStatus";
import { AppShell } from "../shell/AppShell";
import type { RouteNavItem } from "./nav";

export function RootWorkbenchLayout() {
  const workspace = useWorkspaceStatus();
  const navigate = useNavigate();
  const location = useLocation();
  const handleNavigate = useCallback(
    (to: RouteNavItem["to"]) => {
      reportBackgroundPromise(navigate({ to }), "Route navigation", { to });
    },
    [navigate],
  );

  return (
    <>
      <ActiveAccountRuntime enabled={workspace.workspaceStatus !== null} />
      <AppUpdateRuntime />
      <ResourceOnboarding />
      <AppShell
        workspaceStatus={workspace.workspaceStatus}
        workspacePending={workspace.workspacePending}
        workspaceErrorCode={workspace.workspaceErrorCode}
        workspaceErrorMessage={workspace.workspaceErrorMessage}
        restoreFailure={workspace.restoreFailure}
        activePath={location.pathname}
        onOpenWorkspace={workspace.openWorkspace}
        onRetryWorkspaceRestore={workspace.retryWorkspaceRestore}
        onNavigate={handleNavigate}
        language={workspace.language}
        languagePending={workspace.languagePending}
        languageErrorMessage={workspace.languageErrorMessage}
        onChangeLanguage={workspace.changeLanguage}
      >
        <Outlet />
      </AppShell>
    </>
  );
}
