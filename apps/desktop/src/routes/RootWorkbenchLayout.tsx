import { Outlet, useLocation, useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import { useWorkspaceStatus } from "../features/workspace/useWorkspaceStatus";
import { AppShell } from "../shell/AppShell";
import type { RouteNavItem } from "./nav";

export function RootWorkbenchLayout() {
  const workspace = useWorkspaceStatus();
  const navigate = useNavigate();
  const location = useLocation();
  const handleNavigate = useCallback(
    (to: RouteNavItem["to"]) => {
      void navigate({ to });
    },
    [navigate],
  );

  return (
    <AppShell
      workspaceStatus={workspace.workspaceStatus}
      workspacePending={workspace.workspacePending}
      workspaceErrorCode={workspace.workspaceErrorCode}
      workspaceErrorMessage={workspace.workspaceErrorMessage}
      activePath={location.pathname}
      onOpenWorkspace={workspace.openWorkspace}
      onCloseWorkspace={workspace.closeWorkspace}
      onNavigate={handleNavigate}
    >
      <Outlet />
    </AppShell>
  );
}
