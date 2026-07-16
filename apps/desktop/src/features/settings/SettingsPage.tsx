import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel, EmptyState } from "@/components/ui";
import { applyLanguagePreference } from "@/i18n";
import { useToastStore } from "@/stores/toast-store";
import type { GlobalSettingsDto, WorkspaceSettingsDto } from "@/types";

import { useWorkspaceStatus } from "../workspace/useWorkspaceStatus";
import { AccountSettingsSection } from "./components/AccountSettingsSection";
import { FrontendSettingsSection } from "./components/FrontendSettingsSection";
import { GenerationSettingsSection } from "./components/GenerationSettingsSection";
import { ImageSettingsSection } from "./components/ImageSettingsSection";
import { LoadingPanel } from "./components/SettingsControls";
import { SettingsSectionNav, type SettingsSection } from "./components/SettingsSectionNav";
import { WorkspaceLifecycleSection } from "./components/WorkspaceLifecycleSection";
import {
  useGlobalSettingsQuery,
  useUpdateGlobalSettingsMutation,
} from "./data/useGlobalSettingsQuery";
import {
  useResetWorkspaceSettingsMutation,
  useUpdateWorkspaceSettingsMutation,
  useWorkspaceSettingsQuery,
} from "./data/useWorkspaceSettingsQuery";
import { cloneGlobalSettings, cloneSettings, formatError } from "./settings-utils";

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const workspace = useWorkspaceStatus();
  const workspaceSettingsQuery = useWorkspaceSettingsQuery();
  const globalSettingsQuery = useGlobalSettingsQuery();
  const updateWorkspaceMutation = useUpdateWorkspaceSettingsMutation();
  const resetWorkspaceMutation = useResetWorkspaceSettingsMutation();
  const updateGlobalMutation = useUpdateGlobalSettingsMutation();
  const [activeSection, setActiveSection] = useState<SettingsSection>("account");
  const [workspaceDraft, setWorkspaceDraft] = useState<WorkspaceSettingsDto | null>(null);
  const [globalDraft, setGlobalDraft] = useState<GlobalSettingsDto | null>(null);

  useEffect(() => {
    if (workspaceSettingsQuery.data) {
      setWorkspaceDraft(cloneSettings(workspaceSettingsQuery.data));
    }
  }, [workspaceSettingsQuery.data]);

  useEffect(() => {
    if (globalSettingsQuery.data) {
      setGlobalDraft(cloneGlobalSettings(globalSettingsQuery.data));
    }
  }, [globalSettingsQuery.data]);

  const saveGenerationSettings = useCallback(
    (settings: WorkspaceSettingsDto) => {
      if (!workspaceSettingsQuery.data) {
        pushToast({ level: "error", message: t("workspaceNotLoaded") });
        return;
      }
      const nextSettings = cloneSettings(workspaceSettingsQuery.data);
      nextSettings.generation = {
        ...settings.generation,
        size: { ...settings.generation.size },
      };
      saveWorkspaceSettings(nextSettings, updateWorkspaceMutation, setWorkspaceDraft, pushToast, t);
    },
    [pushToast, t, updateWorkspaceMutation, workspaceSettingsQuery.data],
  );

  const saveImageSettings = useCallback(
    (settings: WorkspaceSettingsDto) => {
      if (!workspaceSettingsQuery.data) {
        pushToast({ level: "error", message: t("workspaceNotLoaded") });
        return;
      }
      const nextSettings = cloneSettings(workspaceSettingsQuery.data);
      nextSettings.image_variants = { ...settings.image_variants };
      saveWorkspaceSettings(nextSettings, updateWorkspaceMutation, setWorkspaceDraft, pushToast, t);
    },
    [pushToast, t, updateWorkspaceMutation, workspaceSettingsQuery.data],
  );

  const saveFrontendSettings = useCallback(
    (settings: GlobalSettingsDto) => {
      updateGlobalMutation.mutate(
        { frontend: settings.frontend },
        {
          onSuccess: (updatedSettings) => {
            setGlobalDraft(cloneGlobalSettings(updatedSettings));
            void applyLanguagePreference(updatedSettings.frontend.language);
            pushToast({ level: "success", message: t("settingsSaved") });
          },
          onError: (error) => {
            pushToast({
              level: "error",
              title: t("settingsSaveFailed"),
              message: formatError(error),
            });
          },
        },
      );
    },
    [pushToast, t, updateGlobalMutation],
  );

  const resetSettings = useCallback(() => {
    resetWorkspaceMutation.mutate(undefined, {
      onSuccess: (response) => {
        setWorkspaceDraft(cloneSettings(response.settings));
        pushToast({ level: "success", message: t("settingsReset") });
      },
      onError: (error) => {
        pushToast({ level: "error", title: t("settingsResetFailed"), message: formatError(error) });
      },
    });
  }, [pushToast, resetWorkspaceMutation, t]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="grid min-h-0 flex-1 grid-cols-[220px_minmax(0,1fr)] divide-x divide-app-border">
        <SettingsSectionNav activeSection={activeSection} onSelect={setActiveSection} />
        <SettingsContent
          activeSection={activeSection}
          workspace={workspace.workspaceStatus}
          closeWorkspace={workspace.closeWorkspace}
          closingWorkspace={workspace.closingWorkspace}
          workspaceDraft={workspaceDraft}
          globalDraft={globalDraft}
          workspacePending={workspaceSettingsQuery.isPending}
          globalPending={globalSettingsQuery.isPending}
          workspaceError={
            workspaceSettingsQuery.isError ? formatError(workspaceSettingsQuery.error) : null
          }
          globalError={globalSettingsQuery.isError ? formatError(globalSettingsQuery.error) : null}
          savingWorkspace={updateWorkspaceMutation.isPending}
          savingGlobal={updateGlobalMutation.isPending}
          resetting={resetWorkspaceMutation.isPending}
          updateWorkspaceDraft={setWorkspaceDraft}
          updateGlobalDraft={setGlobalDraft}
          saveGenerationSettings={saveGenerationSettings}
          saveImageSettings={saveImageSettings}
          saveFrontendSettings={saveFrontendSettings}
          resetSettings={resetSettings}
        />
      </div>
    </div>
  );
}

type WorkspaceSettingsMutation = ReturnType<typeof useUpdateWorkspaceSettingsMutation>;

function saveWorkspaceSettings(
  settings: WorkspaceSettingsDto,
  mutation: WorkspaceSettingsMutation,
  setDraft: (settings: WorkspaceSettingsDto) => void,
  pushToast: ReturnType<typeof useToastStore.getState>["push"],
  t: ReturnType<typeof useTranslation<"settings">>["t"],
) {
  mutation.mutate(
    { settings },
    {
      onSuccess: (updatedSettings) => {
        setDraft(cloneSettings(updatedSettings));
        pushToast({ level: "success", message: t("settingsSaved") });
      },
      onError: (error) =>
        pushToast({ level: "error", title: t("settingsSaveFailed"), message: formatError(error) }),
    },
  );
}

function SettingsContent({
  activeSection,
  workspace,
  closeWorkspace,
  closingWorkspace,
  workspaceDraft,
  globalDraft,
  workspacePending,
  globalPending,
  workspaceError,
  globalError,
  savingWorkspace,
  savingGlobal,
  resetting,
  updateWorkspaceDraft,
  updateGlobalDraft,
  saveGenerationSettings,
  saveImageSettings,
  saveFrontendSettings,
  resetSettings,
}: SettingsContentProps) {
  const { t } = useTranslation("settings");
  if (activeSection === "account") {
    return <AccountSettingsSection />;
  }

  if (activeSection === "workspace") {
    return workspace ? (
      <WorkspaceLifecycleSection
        workspace={workspace}
        closeWorkspace={closeWorkspace}
        closing={closingWorkspace}
      />
    ) : (
      <SettingsUnavailable description={t("noWorkspace")} title={t("settingsUnavailable")} />
    );
  }

  if (activeSection === "frontend") {
    if (globalError) {
      return <SettingsUnavailable description={globalError} title={t("settingsUnavailable")} />;
    }
    if (globalPending || !globalDraft) {
      return <SettingsLoading label={t("loadingApplication")} />;
    }
    return (
      <FrontendSettingsSection
        draft={globalDraft}
        updateDraft={updateGlobalDraft}
        saveSettings={saveFrontendSettings}
        saving={savingGlobal}
      />
    );
  }

  if (workspaceError) {
    return <SettingsUnavailable description={workspaceError} title={t("settingsUnavailable")} />;
  }
  if (workspacePending || !workspaceDraft) {
    return <SettingsLoading label={t("loadingWorkspace")} />;
  }
  if (activeSection === "generation") {
    return (
      <GenerationSettingsSection
        draft={workspaceDraft}
        updateDraft={updateWorkspaceDraft}
        saveSettings={saveGenerationSettings}
        resetSettings={resetSettings}
        saving={savingWorkspace}
        resetting={resetting}
      />
    );
  }
  return (
    <ImageSettingsSection
      draft={workspaceDraft}
      updateDraft={updateWorkspaceDraft}
      saveSettings={saveImageSettings}
      saving={savingWorkspace}
    />
  );
}

type SettingsContentProps = {
  activeSection: SettingsSection;
  workspace: ReturnType<typeof useWorkspaceStatus>["workspaceStatus"];
  closeWorkspace: () => void;
  closingWorkspace: boolean;
  workspaceDraft: WorkspaceSettingsDto | null;
  globalDraft: GlobalSettingsDto | null;
  workspacePending: boolean;
  globalPending: boolean;
  workspaceError: string | null;
  globalError: string | null;
  savingWorkspace: boolean;
  savingGlobal: boolean;
  resetting: boolean;
  updateWorkspaceDraft: (draft: WorkspaceSettingsDto) => void;
  updateGlobalDraft: (draft: GlobalSettingsDto) => void;
  saveGenerationSettings: (settings: WorkspaceSettingsDto) => void;
  saveImageSettings: (settings: WorkspaceSettingsDto) => void;
  saveFrontendSettings: (settings: GlobalSettingsDto) => void;
  resetSettings: () => void;
};

function SettingsUnavailable({ title, description }: { title: string; description: string }) {
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden">
      <EmptyState title={title} description={description} />
    </AppPanel>
  );
}

function SettingsLoading({ label }: { label: string }) {
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden">
      <LoadingPanel label={label} />
    </AppPanel>
  );
}
