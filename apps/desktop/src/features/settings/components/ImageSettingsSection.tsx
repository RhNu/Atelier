import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel } from "@/components/ui";
import type { WorkspaceSettingsDto } from "@/types";

import { isPositiveInteger } from "../settings-utils";
import { NumberField, SectionHeader } from "./SettingsControls";

export function ImageSettingsSection({
  draft,
  updateDraft,
}: {
  draft: WorkspaceSettingsDto;
  updateDraft: (draft: WorkspaceSettingsDto) => void;
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
  return (
    <AppPanel variant="section" className="flex h-full min-h-0 flex-col overflow-hidden">
      <SectionHeader title={t("images")} />
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
    </AppPanel>
  );
}
