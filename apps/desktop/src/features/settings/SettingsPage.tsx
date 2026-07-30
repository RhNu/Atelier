import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { reportBackgroundPromise } from "@/app/logger";
import { AppPanel, EmptyState } from "@/components/ui";
import { applyLanguagePreference } from "@/i18n";
import { useToastStore } from "@/stores/toast-store";
import type { GlobalSettingsDto, WorkspaceSettingsDto } from "@/types";

import { useWorkspaceStatus } from "../workspace/useWorkspaceStatus";
import { ConnectionsSettingsSection } from "./components/ConnectionsSettingsSection";
import { FrontendSettingsSection } from "./components/FrontendSettingsSection";
import { GenerationSettingsSection } from "./components/GenerationSettingsSection";
import { ImageSettingsSection } from "./components/ImageSettingsSection";
import { SafetySettingsSection } from "./components/SafetySettingsSection";
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
  const [activeSection, setActiveSection] = useState<SettingsSection>("connections");
  const [workspaceDraft, setWorkspaceDraft] = useState<WorkspaceSettingsDto | null>(null);
  const [globalDraft, setGlobalDraft] = useState<GlobalSettingsDto | null>(null);

  useEffect(() => {
    if (workspaceSettingsQuery.data && workspaceDraft === null) {
      setWorkspaceDraft(cloneSettings(workspaceSettingsQuery.data));
    }
  }, [workspaceDraft, workspaceSettingsQuery.data]);

  useEffect(() => {
    if (globalSettingsQuery.data && globalDraft === null) {
      setGlobalDraft(cloneGlobalSettings(globalSettingsQuery.data));
    }
  }, [globalDraft, globalSettingsQuery.data]);

  useEffect(() => {
    const draft = workspaceDraft;
    const saved = workspaceSettingsQuery.data;
    if (!draft || !saved) return;

    const generationChanged = JSON.stringify(draft.generation) !== JSON.stringify(saved.generation);
    const imageVariantsChanged =
      JSON.stringify(draft.image_variants) !== JSON.stringify(saved.image_variants);
    const imageVariantsValid =
      Number.isInteger(draft.image_variants.thumbnail_long_edge) &&
      draft.image_variants.thumbnail_long_edge > 0 &&
      Number.isInteger(draft.image_variants.preview_long_edge) &&
      draft.image_variants.preview_long_edge > 0;
    if (!generationChanged && (!imageVariantsChanged || !imageVariantsValid)) return;

    const timer = window.setTimeout(() => {
      const nextSettings = cloneSettings(saved);
      if (generationChanged) {
        nextSettings.generation = {
          ...draft.generation,
          size: { ...draft.generation.size },
        };
      }
      if (imageVariantsChanged && imageVariantsValid) {
        nextSettings.image_variants = { ...draft.image_variants };
      }
      updateWorkspaceMutation.mutate(
        { settings: nextSettings },
        {
          onError: (error) =>
            pushToast({
              level: "error",
              title: t("settingsSaveFailed"),
              message: formatError(error),
            }),
        },
      );
    }, SETTINGS_AUTOSAVE_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [pushToast, t, updateWorkspaceMutation, workspaceDraft, workspaceSettingsQuery.data]);

  useEffect(() => {
    const draft = globalDraft;
    const saved = globalSettingsQuery.data;
    if (!draft || !saved) return;
    if (
      JSON.stringify(draft.frontend) === JSON.stringify(saved.frontend) &&
      JSON.stringify(draft.safety) === JSON.stringify(saved.safety)
    )
      return;

    const timer = window.setTimeout(() => {
      updateGlobalMutation.mutate(
        { frontend: draft.frontend, safety: draft.safety },
        {
          onSuccess: (updatedSettings) => {
            reportBackgroundPromise(
              applyLanguagePreference(updatedSettings.frontend.language),
              "Apply updated language preference",
            );
          },
          onError: (error) =>
            pushToast({
              level: "error",
              title: t("settingsSaveFailed"),
              message: formatError(error),
            }),
        },
      );
    }, SETTINGS_AUTOSAVE_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [globalDraft, globalSettingsQuery.data, pushToast, t, updateGlobalMutation]);

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
          resetting={resetWorkspaceMutation.isPending}
          updateWorkspaceDraft={setWorkspaceDraft}
          updateGlobalDraft={setGlobalDraft}
          resetSettings={resetSettings}
        />
      </div>
    </div>
  );
}

const SETTINGS_AUTOSAVE_DELAY_MS = 500;

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
  resetting,
  updateWorkspaceDraft,
  updateGlobalDraft,
  resetSettings,
}: SettingsContentProps) {
  const { t } = useTranslation("settings");
  if (activeSection === "connections") {
    return <ConnectionsSettingsSection />;
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

  if (activeSection === "frontend" || activeSection === "safety") {
    if (globalError) {
      return <SettingsUnavailable description={globalError} title={t("settingsUnavailable")} />;
    }
    if (globalPending || !globalDraft) {
      return <SettingsLoading label={t("loadingApplication")} />;
    }
    return activeSection === "frontend" ? (
      <FrontendSettingsSection draft={globalDraft} updateDraft={updateGlobalDraft} />
    ) : (
      <SafetySettingsSection draft={globalDraft} updateDraft={updateGlobalDraft} />
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
        resetSettings={resetSettings}
        resetting={resetting}
      />
    );
  }
  return <ImageSettingsSection draft={workspaceDraft} updateDraft={updateWorkspaceDraft} />;
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
  resetting: boolean;
  updateWorkspaceDraft: (draft: WorkspaceSettingsDto) => void;
  updateGlobalDraft: (draft: GlobalSettingsDto) => void;
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
