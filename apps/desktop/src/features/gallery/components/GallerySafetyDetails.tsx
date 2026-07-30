import { RotateCw, ShieldCheck } from "lucide-react";
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
  onRescan: () => void;
  applyingOverride: boolean;
  rescanning: boolean;
};

export function GallerySafetyDetails({
  item,
  overrideValue,
  onOverrideChange,
  onApplyOverride,
  onRescan,
  applyingOverride,
  rescanning,
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
  const assessment = item.safety;
  const primary = assessment?.primary;
  const review = assessment?.review;

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
        {primary ? (
          <span className="shrink-0 text-xs text-app-muted">
            {t("fusedScore", { value: primary.fused_score.toFixed(3) })}
          </span>
        ) : null}
      </div>
      {assessment && primary ? (
        <div className="grid gap-2 border border-app-border bg-black/15 p-3 text-xs text-app-muted">
          <p>
            <span className="text-app-text">{t("manualSafetyOverride")}:</span>{" "}
            {item.manual_safety_override
              ? t(item.manual_safety_override)
              : t("automaticSafetyDecision")}
          </p>
          <p>
            <span className="text-app-text">{t("safetyPolicy")}:</span> {assessment.policy_id}@
            {assessment.policy_version}
          </p>
          <EvidenceRow title={t("primaryEvidence")} evidence={primary} />
          <div>
            <span className="text-app-text">{t("reviewEvidence")}:</span>{" "}
            {t(`reviewStates.${review?.state ?? "not_needed"}`)}
            {review?.evidence ? <EvidenceRow evidence={review.evidence} /> : null}
            {review?.message ? <p className="mt-1 text-rose-200">{review.message}</p> : null}
          </div>
        </div>
      ) : null}
      {assessment.scan_state !== "scanned" ? (
        <div className="grid gap-2 border border-app-border bg-black/15 p-3 text-xs text-app-muted">
          <p>{t(`scanStates.${assessment.scan_state}`)}</p>
          {assessment.message ? <p className="text-rose-200">{assessment.message}</p> : null}
          <div>
            <AppButton variant="secondary" disabled={rescanning} onClick={onRescan}>
              <RotateCw
                aria-hidden="true"
                className={rescanning ? "size-4 animate-spin" : "size-4"}
              />
              {rescanning ? t("rescanningSafety") : t("rescanSafety")}
            </AppButton>
          </div>
        </div>
      ) : null}
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

function EvidenceRow({
  title,
  evidence,
}: {
  title?: string;
  evidence: NonNullable<NonNullable<GalleryItemDto["safety"]>["primary"]>;
}) {
  const { t } = useTranslation("gallery");
  return (
    <div className="grid gap-1">
      <p>
        {title ? <span className="text-app-text">{title}: </span> : null}
        {evidence.model_id}@{evidence.model_revision.slice(0, 12)} ·{" "}
        {t("fusedScore", { value: evidence.fused_score.toFixed(3) })}
      </p>
      <p>
        {t("ratingScores", {
          general: evidence.ratings.general.toFixed(3),
          sensitive: evidence.ratings.sensitive.toFixed(3),
          questionable: evidence.ratings.questionable.toFixed(3),
          explicit: evidence.ratings.explicit.toFixed(3),
        })}
      </p>
    </div>
  );
}
