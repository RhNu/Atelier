/* eslint-disable react-perf/jsx-no-jsx-as-prop */
import { Outlet, useLocation, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect } from "react";

import { reportBackgroundPromise } from "../app/logger";
import { ActiveAccountRuntime } from "../features/account/runtime/ActiveAccountRuntime";
import { AgentDrawer, useAgentDrawerStore } from "../features/agent";
import { PromptEditorSettingsProvider } from "../features/prompt-editor";
import { AppUpdateRuntime } from "../features/settings/components/AppUpdateRuntime";
import { ResourceOnboarding } from "../features/settings/components/ResourceOnboarding";
import { useWorkspaceStatus } from "../features/workspace/useWorkspaceStatus";
import { AppShell } from "../shell/AppShell";
import type { RouteNavItem } from "./nav";

export function RootWorkbenchLayout() {
  const workspace = useWorkspaceStatus();
  const navigate = useNavigate();
  const location = useLocation();
  const agentOpen = useAgentDrawerStore((state) => state.open);
  const agentRunning = useAgentDrawerStore((state) => state.runningSessionId !== null);
  const toggleAgent = useAgentDrawerStore((state) => state.toggle);
  const resetAgentWorkspace = useAgentDrawerStore((state) => state.resetWorkspace);
  const handleNavigate = useCallback(
    (to: RouteNavItem["to"]) => {
      reportBackgroundPromise(navigate({ to }), "Route navigation", { to });
    },
    [navigate],
  );
  const openAgentSettings = useCallback(() => {
    window.sessionStorage.setItem("atelier.settings.section", "agent");
    handleNavigate("/settings");
  }, [handleNavigate]);

  useEffect(() => {
    if (!workspace.workspaceStatus) resetAgentWorkspace();
  }, [resetAgentWorkspace, workspace.workspaceStatus]);

  const agentDrawer = workspace.workspaceStatus ? (
    <AgentDrawer
      route={location.pathname}
      workspaceId={workspace.workspaceStatus.root}
      onOpenSettings={openAgentSettings}
    />
  ) : null;

  return (
    <PromptEditorSettingsProvider
      convertFullWidthPunctuation={workspace.convertFullWidthPunctuation ?? false}
    >
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
        agentOpen={agentOpen}
        agentRunning={agentRunning}
        onToggleAgent={toggleAgent}
        agentDrawer={agentDrawer}
      >
        <Outlet />
      </AppShell>
    </PromptEditorSettingsProvider>
  );
}
