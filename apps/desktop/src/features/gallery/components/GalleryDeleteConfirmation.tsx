import { AppButton, AppModal } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

export function GalleryDeleteConfirmation({
  targetIds,
  targets,
  deleting,
  error,
  onClose,
  onConfirm,
}: {
  targetIds: string[];
  targets: GalleryItemDto[];
  deleting: boolean;
  error: string | null;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation("gallery");
  const { t: translateCommon } = useTranslation("common");
  const targetId = targetIds[0] ?? null;
  const target = targets[0] ?? null;
  const isBatch = targetIds.length > 1;
  return (
    <AppModal
      open={targetIds.length > 0}
      title={isBatch ? t("deleteBatchTitle") : t("deleteTitle")}
      onClose={onClose}
    >
      <div className="grid gap-4">
        <div className="grid gap-2 text-sm text-app-text">
          {isBatch ? (
            <p>{t("deleteBatchPrompt", { count: targetIds.length })}</p>
          ) : (
            <p>
              {t("deleteBefore")} <span className="font-semibold text-white">{targetId}</span>{" "}
              {t("deleteAfter")}
            </p>
          )}
          <p className="text-app-muted">{t("deleteWarning")}</p>
          {!isBatch && target ? (
            <p className="text-xs text-app-muted">{t("artifactId", { id: target.artifact_id })}</p>
          ) : null}
        </div>
        {error ? (
          <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
            {error}
          </p>
        ) : null}
        <div className="flex justify-end gap-2">
          <AppButton variant="secondary" onClick={onClose} disabled={deleting}>
            {translateCommon("cancel")}
          </AppButton>
          <AppButton variant="danger" onClick={onConfirm} disabled={deleting}>
            {t("deletePermanently")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}
import { useTranslation } from "react-i18next";
