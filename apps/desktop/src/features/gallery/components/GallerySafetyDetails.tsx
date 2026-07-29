import { ShieldCheck } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, AppModal, AppSelect, SafetyBadge } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

import { effectiveSafetyLabel, overrideOptions } from "../gallery-utils";

type GallerySafetyDetailsProps = {
  item: GalleryItemDto;
  overrideValue: string;
  onOverrideChange: (value: string) => void;
  onApplyOverride: () => void;
  applyingOverride: boolean;
};

export function GallerySafetyDetails({
  item,
  overrideValue,
  onOverrideChange,
  onApplyOverride,
  applyingOverride,
}: GallerySafetyDetailsProps) {
  const { t } = useTranslation("gallery");
  const [overrideDialogOpen, setOverrideDialogOpen] = useState(false);
  const safetyLabel = effectiveSafetyLabel(item);
  const localizedOverrides = useMemo(
    () => overrideOptions.map((option) => ({ ...option, label: t(option.labelKey) })),
    [t],
  );
  const openOverrideDialog = useCallback(() => setOverrideDialogOpen(true), []);
  const closeOverrideDialog = useCallback(() => setOverrideDialogOpen(false), []);
  const handleApplyOverride = useCallback(() => {
    onApplyOverride();
    closeOverrideDialog();
  }, [closeOverrideDialog, onApplyOverride]);
  const nsfwScore = item.safety?.nsfw_score?.toFixed(2) ?? null;
  const safeScore = item.safety?.safe_score?.toFixed(2) ?? null;

  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">{t("safety")}</h3>
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <SafetyBadge
            label={safetyLabel}
            displayLabel={safetyLabel === "unknown" ? t("unknown") : t(safetyLabel)}
          />
          <AppIconButton
            icon={ShieldCheck}
            size="sm"
            label={t("changeSafetyOverride")}
            onClick={openOverrideDialog}
          />
        </div>
        <div className="flex shrink-0 gap-3 text-xs text-app-muted">
          {nsfwScore ? <span>{t("nsfwScore", { value: nsfwScore })}</span> : null}
          {safeScore ? <span>{t("safeScore", { value: safeScore })}</span> : null}
        </div>
      </div>
      <AppModal open={overrideDialogOpen} title={t("safetyOverride")} onClose={closeOverrideDialog}>
        <div className="grid gap-4">
          <label
            htmlFor="gallery-safety-override"
            className="grid gap-1 text-sm font-semibold text-app-text"
          >
            {t("safetyOverride")}
            <AppSelect
              id="gallery-safety-override"
              aria-label={t("safetyOverride")}
              options={localizedOverrides}
              value={overrideValue}
              onValueChange={onOverrideChange}
            />
          </label>
          <div className="flex justify-end gap-2">
            <AppButton variant="ghost" onClick={closeOverrideDialog}>
              {t("cancel")}
            </AppButton>
            <AppButton onClick={handleApplyOverride} disabled={applyingOverride}>
              <ShieldCheck aria-hidden="true" className="size-4" />
              {t("applyOverride")}
            </AppButton>
          </div>
        </div>
      </AppModal>
    </section>
  );
}
