import { Save } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import type { WorkspaceSettingsDto } from "@/types";

import { isPositiveInteger } from "../settings-utils";
import { NumberField, SectionHeader } from "./SettingsControls";

export function ImageSettingsSection({
  draft,
  updateDraft,
  saveSettings,
  saving,
  commandError,
}: {
  draft: WorkspaceSettingsDto;
  updateDraft: (draft: WorkspaceSettingsDto) => void;
  saveSettings: (settings: WorkspaceSettingsDto) => void;
  saving: boolean;
  commandError: string | null;
}) {
  const { t } = useTranslation("settings");
  const variants = draft.image_variants;
  const isValid =
    isPositiveInteger(variants.thumbnail_long_edge) &&
    isPositiveInteger(variants.preview_long_edge);
  const updateVariant = useCallback(
    (key: keyof WorkspaceSettingsDto["image_variants"], value: number) => {
      updateDraft({ ...draft, image_variants: { ...draft.image_variants, [key]: value } });
    },
    [draft, updateDraft],
  );
  const updateThumbnail = useCallback(
    (value: number) => updateVariant("thumbnail_long_edge", value),
    [updateVariant],
  );
  const updatePreview = useCallback(
    (value: number) => updateVariant("preview_long_edge", value),
    [updateVariant],
  );
  const save = useCallback(() => {
    saveSettings(draft);
  }, [draft, saveSettings]);

  return (
    <AppPanel variant="section" className="flex h-full min-h-0 flex-col overflow-hidden">
      <SectionHeader
        kicker={t("images")}
        title={t("imageVariants")}
        description={t("imagesDescriptionLong")}
      >
        <AppButton disabled={saving || !isValid} onClick={save}>
          <Save aria-hidden="true" className="size-4" />
          {t("saveImageVariants")}
        </AppButton>
      </SectionHeader>
      <div className="grid gap-3 p-3 md:grid-cols-2">
        <NumberField
          label={t("thumbnailLongEdge")}
          value={variants.thumbnail_long_edge}
          onChange={updateThumbnail}
        />
        <NumberField
          label={t("previewLongEdge")}
          value={variants.preview_long_edge}
          onChange={updatePreview}
        />
        {!isValid ? (
          <p className="text-sm text-amber-200 md:col-span-2">{t("positiveIntegerRequired")}</p>
        ) : null}
      </div>
      {commandError ? <ImageCommandError message={commandError} /> : null}
    </AppPanel>
  );
}

function ImageCommandError({ message }: { message: string }) {
  return (
    <p className="border-t border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
      {message}
    </p>
  );
}
