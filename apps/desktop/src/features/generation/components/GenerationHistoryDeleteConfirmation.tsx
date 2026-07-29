import { useTranslation } from "react-i18next";

import { AppButton, AppModal } from "@/components/ui";

export function GenerationHistoryDeleteConfirmation({
  count,
  deleting,
  onClose,
  onConfirm,
}: {
  count: number;
  deleting: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation("generation");
  const { t: translateCommon } = useTranslation("common");
  return (
    <AppModal open={count > 0} title={t("deleteHistoryTitle")} onClose={onClose}>
      <div className="grid gap-4">
        <p className="text-sm text-app-text">{t("deleteHistoryPrompt", { count })}</p>
        <p className="text-sm text-app-muted">{t("deleteHistoryWarning")}</p>
        <div className="flex justify-end gap-2">
          <AppButton variant="secondary" onClick={onClose} disabled={deleting}>
            {translateCommon("cancel")}
          </AppButton>
          <AppButton variant="danger" onClick={onConfirm} disabled={deleting}>
            {t("deleteHistoryConfirm")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}
