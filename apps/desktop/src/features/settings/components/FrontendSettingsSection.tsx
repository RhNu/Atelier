import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, LanguageSelect } from "@/components/ui";
import type { GlobalSettingsDto } from "@/types";

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
  const { t } = useTranslation("settings");
  const updateLanguage = useCallback(
    (language: GlobalSettingsDto["frontend"]["language"]) => {
      updateDraft({ ...draft, frontend: { ...draft.frontend, language } });
    },
    [draft, updateDraft],
  );
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
    <AppPanel variant="section" className="h-full min-h-0 overflow-hidden">
      <SectionHeader
        kicker={t("frontend")}
        title={t("frontendTitle")}
        description={t("frontendDescription")}
      >
        <AppButton onClick={handleSave} disabled={saving}>
          {saving ? t("savingFrontend") : t("saveFrontend")}
        </AppButton>
      </SectionHeader>
      {commandError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {commandError}
        </p>
      ) : null}
      <div className="grid gap-3 p-3 md:grid-cols-2">
        <LanguageSelect value={draft.frontend.language} onChange={updateLanguage} />
        <CheckboxField
          label={t("developerMode")}
          checked={draft.frontend.developer_mode}
          onChange={updateDeveloperMode}
        />
        <CheckboxField
          label={t("blurNsfw")}
          checked={draft.frontend.gallery.blur_sensitive_images}
          onChange={updateBlurSensitiveImages}
        />
      </div>
    </AppPanel>
  );
}
