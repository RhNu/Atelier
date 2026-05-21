import { Outlet, useLocation, useNavigate } from "@tanstack/react-router";

import { useWorkspaceStatus } from "../features/workspace/useWorkspaceStatus";
import { AppShell } from "../shell/AppShell";

export function RootWorkbenchLayout() {
  const workspace = useWorkspaceStatus();
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <AppShell
      workspaceStatus={workspace.workspaceStatus}
      workspacePending={workspace.workspacePending}
      workspaceErrorCode={workspace.workspaceErrorCode}
      workspaceErrorMessage={workspace.workspaceErrorMessage}
      activePath={location.pathname}
      onOpenWorkspace={workspace.openWorkspace}
      onCloseWorkspace={workspace.closeWorkspace}
      onNavigate={(to) => void navigate({ to })}
    >
      <Outlet />
    </AppShell>
  );
}
