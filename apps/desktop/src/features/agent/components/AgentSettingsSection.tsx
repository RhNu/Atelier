import { useTranslation } from "react-i18next";

import { AppPanel } from "@/components/ui";
import type { AgentModelDto } from "@/types";

import { LoadingPanel, SectionHeader } from "../../settings/components/SettingsControls";
import { useAgentRegistryQuery, useAgentWorkspaceSettingsQuery } from "../data/useAgentQueries";
import { formatError } from "./agent-settings-shared";
import { AgentConnectionsSettings } from "./AgentConnectionsSettings";
import { AgentModelsSettings } from "./AgentModelsSettings";
import { AgentPersonaSettings } from "./AgentPersonaSettings";

const EMPTY_MODELS: AgentModelDto[] = [];

export function AgentSettingsSection() {
  const { t } = useTranslation("agent");
  const registry = useAgentRegistryQuery();
  const workspaceSettings = useAgentWorkspaceSettingsQuery();
  if (registry.isPending || workspaceSettings.isPending) {
    return <LoadingPanel label={t("settingsLoading")} />;
  }
  if (registry.isError || workspaceSettings.isError) {
    return (
      <div className="p-4 text-sm text-rose-100">
        {formatError(registry.error ?? workspaceSettings.error)}
      </div>
    );
  }
  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <SectionHeader title={t("settingsTitle")} />
      <div className="grid min-h-0 flex-1 content-start gap-6 overflow-auto p-4">
        <AgentConnectionsSettings />
        <AgentModelsSettings />
        {workspaceSettings.data ? (
          <AgentPersonaSettings
            initial={workspaceSettings.data}
            modelOptions={registry.data?.models ?? EMPTY_MODELS}
          />
        ) : null}
      </div>
    </AppPanel>
  );
}
