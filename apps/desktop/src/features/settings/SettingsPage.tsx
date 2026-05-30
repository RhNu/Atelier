import { Settings } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import { AppPanel, AppToolbar, EmptyState } from "../../components/ui";
import type { WorkspaceSettingsDto } from "../../types";
import { AccountSettingsSection } from "./components/AccountSettingsSection";
import { FrontendSettingsSection } from "./components/FrontendSettingsSection";
import { GenerationSettingsSection } from "./components/GenerationSettingsSection";
import { ImageSettingsSection } from "./components/ImageSettingsSection";
import { LoadingPanel } from "./components/SettingsControls";
import { SettingsSectionNav, type SettingsSection } from "./components/SettingsSectionNav";
import {
  useResetWorkspaceSettingsMutation,
  useUpdateWorkspaceSettingsMutation,
  useWorkspaceSettingsQuery,
} from "./data/useWorkspaceSettingsQuery";
import { cloneSettings, formatError } from "./settings-utils";

export function SettingsPage() {
  const settingsQuery = useWorkspaceSettingsQuery();
  const updateSettingsMutation = useUpdateWorkspaceSettingsMutation();
  const resetSettingsMutation = useResetWorkspaceSettingsMutation();
  const [activeSection, setActiveSection] = useState<SettingsSection>("account");
  const [draft, setDraft] = useState<WorkspaceSettingsDto | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);

  useEffect(() => {
    if (settingsQuery.data) {
      setDraft(cloneSettings(settingsQuery.data));
    }
  }, [settingsQuery.data]);

  const updateDraft = useCallback((nextDraft: WorkspaceSettingsDto) => {
    setDraft(nextDraft);
  }, []);

  const saveGenerationSettings = useCallback(
    (settings: WorkspaceSettingsDto) => {
      if (!settingsQuery.data) {
        setCommandError("Workspace settings are not loaded.");
        return;
      }

      const nextSettings = cloneSettings(settingsQuery.data);
      nextSettings.generation = {
        ...settings.generation,
        size: { ...settings.generation.size },
      };

      setCommandError(null);
      updateSettingsMutation.mutate(
        { settings: nextSettings },
        {
          onSuccess: (updatedSettings) => {
            setDraft(cloneSettings(updatedSettings));
          },
          onError: (error) => {
            setCommandError(formatError(error));
          },
        },
      );
    },
    [settingsQuery.data, updateSettingsMutation],
  );

  const saveImageSettings = useCallback(
    (settings: WorkspaceSettingsDto) => {
      if (!settingsQuery.data) {
        setCommandError("Workspace settings are not loaded.");
        return;
      }

      const nextSettings = cloneSettings(settingsQuery.data);
      nextSettings.image_variants = { ...settings.image_variants };

      setCommandError(null);
      updateSettingsMutation.mutate(
        { settings: nextSettings },
        {
          onSuccess: (updatedSettings) => {
            setDraft(cloneSettings(updatedSettings));
          },
          onError: (error) => {
            setCommandError(formatError(error));
          },
        },
      );
    },
    [settingsQuery.data, updateSettingsMutation],
  );

  const saveFrontendSettings = useCallback(
    (settings: WorkspaceSettingsDto) => {
      if (!settingsQuery.data) {
        setCommandError("Workspace settings are not loaded.");
        return;
      }

      const nextSettings = cloneSettings(settingsQuery.data);
      nextSettings.frontend = {
        gallery: { ...settings.frontend.gallery },
      };

      setCommandError(null);
      updateSettingsMutation.mutate(
        { settings: nextSettings },
        {
          onSuccess: (updatedSettings) => {
            setDraft(cloneSettings(updatedSettings));
          },
          onError: (error) => {
            setCommandError(formatError(error));
          },
        },
      );
    },
    [settingsQuery.data, updateSettingsMutation],
  );

  const resetSettings = useCallback(() => {
    setCommandError(null);
    resetSettingsMutation.mutate(undefined, {
      onSuccess: (response) => {
        setDraft(cloneSettings(response.settings));
      },
      onError: (error) => {
        setCommandError(formatError(error));
      },
    });
  }, [resetSettingsMutation]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Settings</p>
          <h1 className="text-lg font-semibold text-white">Workspace Settings</h1>
        </div>
        <div className="flex items-center gap-2 text-sm text-app-muted">
          <Settings aria-hidden="true" className="size-4" />
          NovelAI workspace configuration
        </div>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[220px_minmax(0,1fr)] gap-3 p-3">
        <SettingsSectionNav activeSection={activeSection} onSelect={setActiveSection} />
        <SettingsContent
          activeSection={activeSection}
          draft={draft}
          pending={settingsQuery.isPending}
          error={settingsQuery.isError ? formatError(settingsQuery.error) : null}
          saving={updateSettingsMutation.isPending}
          resetting={resetSettingsMutation.isPending}
          commandError={commandError}
          updateDraft={updateDraft}
          saveGenerationSettings={saveGenerationSettings}
          saveImageSettings={saveImageSettings}
          saveFrontendSettings={saveFrontendSettings}
          resetSettings={resetSettings}
        />
      </div>
    </div>
  );
}

function SettingsContent({
  activeSection,
  draft,
  pending,
  error,
  saving,
  resetting,
  commandError,
  updateDraft,
  saveGenerationSettings,
  saveImageSettings,
  saveFrontendSettings,
  resetSettings,
}: {
  activeSection: SettingsSection;
  draft: WorkspaceSettingsDto | null;
  pending: boolean;
  error: string | null;
  saving: boolean;
  resetting: boolean;
  commandError: string | null;
  updateDraft: (draft: WorkspaceSettingsDto) => void;
  saveGenerationSettings: (settings: WorkspaceSettingsDto) => void;
  saveImageSettings: (settings: WorkspaceSettingsDto) => void;
  saveFrontendSettings: (settings: WorkspaceSettingsDto) => void;
  resetSettings: () => void;
}) {
  if (activeSection === "account") {
    return <AccountSettingsSection />;
  }

  if (error) {
    return (
      <AppPanel className="min-h-0 overflow-hidden">
        <EmptyState title="Workspace settings unavailable" description={error} />
      </AppPanel>
    );
  }

  if (pending || !draft) {
    return (
      <AppPanel className="min-h-0 overflow-hidden">
        <LoadingPanel label="Loading workspace settings" />
      </AppPanel>
    );
  }

  if (activeSection === "frontend") {
    return (
      <FrontendSettingsSection
        draft={draft}
        updateDraft={updateDraft}
        saveSettings={saveFrontendSettings}
        saving={saving}
        commandError={commandError}
      />
    );
  }

  if (activeSection === "generation") {
    return (
      <GenerationSettingsSection
        draft={draft}
        updateDraft={updateDraft}
        saveSettings={saveGenerationSettings}
        resetSettings={resetSettings}
        saving={saving}
        resetting={resetting}
        commandError={commandError}
      />
    );
  }

  return (
    <ImageSettingsSection
      draft={draft}
      updateDraft={updateDraft}
      saveSettings={saveImageSettings}
      saving={saving}
      commandError={commandError}
    />
  );
}
