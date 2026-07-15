import { useCallback } from "react";

import { AppButton, AppPanel } from "../../../components/ui";
import type { GlobalSettingsDto } from "../../../types";
import { CheckboxField, SectionHeader } from "./SettingsControls";

type FrontendSettingsSectionProps = {
  draft: GlobalSettingsDto;
  updateDraft: (draft: GlobalSettingsDto) => void;
  saveSettings: (settings: GlobalSettingsDto) => void;
  saving: boolean;
  commandError: string | null;
};

export function FrontendSettingsSection({
  draft,
  updateDraft,
  saveSettings,
  saving,
  commandError,
}: FrontendSettingsSectionProps) {
  const updateDeveloperMode = useCallback(
    (developerMode: boolean) => {
      updateDraft({
        ...draft,
        frontend: {
          ...draft.frontend,
          developer_mode: developerMode,
        },
      });
    },
    [draft, updateDraft],
  );
  const updateBlurSensitiveImages = useCallback(
    (blurSensitiveImages: boolean) => {
      updateDraft({
        ...draft,
        frontend: {
          ...draft.frontend,
          gallery: {
            ...draft.frontend.gallery,
            blur_sensitive_images: blurSensitiveImages,
          },
        },
      });
    },
    [draft, updateDraft],
  );

  const handleSave = useCallback(() => {
    saveSettings(draft);
  }, [draft, saveSettings]);

  return (
    <AppPanel className="h-full min-h-0 overflow-hidden">
      <SectionHeader
        kicker="Frontend"
        title="Frontend Preferences"
        description="Application-wide interface behavior shared across Atelier workspaces."
      >
        <AppButton onClick={handleSave} disabled={saving}>
          {saving ? "Saving frontend preferences" : "Save frontend preferences"}
        </AppButton>
      </SectionHeader>
      {commandError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {commandError}
        </p>
      ) : null}
      <div className="grid gap-3 p-3 md:grid-cols-2">
        <CheckboxField
          label="Developer mode"
          checked={draft.frontend.developer_mode}
          onChange={updateDeveloperMode}
        />
        <CheckboxField
          label="Blur NSFW images"
          checked={draft.frontend.gallery.blur_sensitive_images}
          onChange={updateBlurSensitiveImages}
        />
      </div>
    </AppPanel>
  );
}
