import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel, LanguageSelect } from "@/components/ui";
import type { GlobalSettingsDto } from "@/types";

import { CheckboxField, SectionHeader } from "./SettingsControls";

type FrontendSettingsSectionProps = {
  draft: GlobalSettingsDto;
  updateDraft: (draft: GlobalSettingsDto) => void;
};

export function FrontendSettingsSection({ draft, updateDraft }: FrontendSettingsSectionProps) {
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

  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-hidden">
      <SectionHeader title={t("interface")} />
      <div className="grid gap-4 p-3">
        <section className="grid gap-2">
          <LanguageSelect value={draft.frontend.language} onChange={updateLanguage} />
        </section>
        <section className="grid gap-2">
          <h3 className="text-xs font-semibold text-app-muted uppercase">{t("displayGroup")}</h3>
          <CheckboxField
            label={t("blurNsfw")}
            checked={draft.frontend.gallery.blur_sensitive_images}
            onChange={updateBlurSensitiveImages}
          />
        </section>
        <section className="grid gap-2">
          <h3 className="text-xs font-semibold text-app-muted uppercase">{t("developerGroup")}</h3>
          <CheckboxField
            label={t("developerMode")}
            checked={draft.frontend.developer_mode}
            onChange={updateDeveloperMode}
          />
        </section>
      </div>
    </AppPanel>
  );
}
