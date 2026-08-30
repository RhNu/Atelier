import { Download } from "lucide-react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { GlobalSettingsDto } from "@/types";

import {
  useDownloadableResourcesQuery,
  useInstallDownloadableResourceMutation,
} from "../data/useDownloadableResources";
import { formatError } from "../settings-utils";
import { CheckboxField, SectionHeader } from "./SettingsControls";

type Props = {
  draft: GlobalSettingsDto;
  updateDraft: (draft: GlobalSettingsDto) => void;
};

export function SafetySettingsSection({ draft, updateDraft }: Props) {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const resources = useDownloadableResourcesQuery();
  const installMutation = useInstallDownloadableResourceMutation();
  const wd = useMemo(
    () => resources.data?.resources.find((resource) => resource.id === "wd-swinv2-tagger-v3"),
    [resources.data],
  );
  const updateAutoReview = useCallback(
    (enabled: boolean) => {
      updateDraft({ ...draft, safety: { ...draft.safety, wd_auto_review_enabled: enabled } });
    },
    [draft, updateDraft],
  );
  const installWd = useCallback(() => {
    installMutation.mutate(
      { request: { resource_id: "wd-swinv2-tagger-v3" }, onProgress: () => undefined },
      {
        onError: (error) =>
          pushToast({
            level: "error",
            title: t("resourceActionFailed"),
            message: formatError(error),
          }),
      },
    );
  }, [installMutation, pushToast, t]);

  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
      <SectionHeader title={t("safety")} />
      <div className="grid gap-4 p-3">
        <p className="text-sm leading-5 text-app-muted">{t("safetyDescriptionLong")}</p>
        <section className="grid gap-3 border border-app-border bg-black/15 p-3">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h3 className="text-sm font-semibold text-app-text">{t("wdModel")}</h3>
              <p className="mt-1 text-xs leading-4 text-app-muted">{t("wdDescription")}</p>
            </div>
            {wd?.state !== "ready" ? (
              <AppButton
                variant="secondary"
                disabled={installMutation.isPending}
                onClick={installWd}
              >
                <Download aria-hidden="true" className="size-4" />
                {t("downloadResource")}
              </AppButton>
            ) : null}
          </div>
          <CheckboxField
            label={t("wdAutoReview")}
            checked={draft.safety.wd_auto_review_enabled}
            disabled={wd?.state !== "ready"}
            onChange={updateAutoReview}
          />
          <p className="text-xs leading-4 text-app-muted">{t("wdAutoReviewDescription")}</p>
        </section>
      </div>
    </AppPanel>
  );
}
