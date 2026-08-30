import { Download, RefreshCw } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import type { AppUpdateProgressDto } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";

import { useAppUpdateQuery, useInstallAppUpdateMutation } from "../data/useAppUpdate";
import { formatError } from "../settings-utils";
import { SectionHeader } from "./SettingsControls";

export function UpdatesSettingsSection() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const update = useAppUpdateQuery();
  const install = useInstallAppUpdateMutation();
  const [progress, setProgress] = useState<AppUpdateProgressDto | null>(null);
  const percent = progress?.total_bytes
    ? Math.min(100, (progress.downloaded_bytes / progress.total_bytes) * 100)
    : 0;
  const progressStyle = useMemo(() => ({ width: `${percent}%` }), [percent]);
  const installUpdate = useCallback(() => {
    install.mutate(setProgress, {
      onError: (error) =>
        pushToast({ level: "error", title: t("updateFailed"), message: formatError(error) }),
    });
  }, [install, pushToast, t]);
  const checkForUpdates = useCallback(() => void update.refetch(), [update]);
  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
      <SectionHeader title={t("updates")} />
      <div className="grid gap-4 p-3">
        <div className="flex items-start justify-between gap-4 border border-app-border bg-black/15 p-4">
          <div>
            <h3 className="text-sm font-semibold text-app-text">
              {update.isError
                ? t("updateCheckFailed")
                : update.data
                  ? t("updateVersion", { version: update.data.version })
                  : t("upToDate")}
            </h3>
            <p className="mt-1 text-xs text-app-muted">
              {update.isError
                ? formatError(update.error)
                : update.data
                  ? t("currentVersion", { version: update.data.current_version })
                  : t("noUpdateDescription")}
            </p>
          </div>
          <div className="flex gap-2">
            <AppButton variant="secondary" disabled={update.isFetching} onClick={checkForUpdates}>
              <RefreshCw aria-hidden="true" className="size-4" /> {t("checkForUpdates")}
            </AppButton>
            {update.data ? (
              <AppButton disabled={install.isPending} onClick={installUpdate}>
                <Download aria-hidden="true" className="size-4" /> {t("installUpdate")}
              </AppButton>
            ) : null}
          </div>
        </div>
        {update.data?.release_notes ? (
          <section className="border border-app-border p-4">
            <h3 className="text-sm font-semibold text-app-text">{t("releaseNotes")}</h3>
            <p className="mt-2 text-sm leading-5 whitespace-pre-wrap text-app-muted">
              {update.data.release_notes}
            </p>
          </section>
        ) : null}
        {install.isPending ? (
          <div className="grid gap-1">
            <div className="h-1.5 overflow-hidden bg-app-surface">
              <div className="h-full bg-brand-400" style={progressStyle} />
            </div>
            <p className="text-xs text-app-muted">{t("installingUpdate")}</p>
          </div>
        ) : null}
      </div>
    </AppPanel>
  );
}
