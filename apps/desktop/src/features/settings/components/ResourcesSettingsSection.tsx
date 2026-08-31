import { Download, Loader2, RefreshCw, Trash2, X } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type {
  DownloadableResourceInstallProgressDto,
  DownloadableResourceStatusDto,
} from "@/types";

import {
  useCancelDownloadableResourceMutation,
  useDeleteDownloadableResourceMutation,
  useDownloadableResourcesQuery,
  useInstallDownloadableResourceMutation,
  useRefreshDownloadableResourcesMutation,
} from "../data/useDownloadableResources";
import { formatError } from "../settings-utils";
import { LoadingPanel, SectionHeader } from "./SettingsControls";

export function ResourcesSettingsSection() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const resourcesQuery = useDownloadableResourcesQuery();
  const refreshMutation = useRefreshDownloadableResourcesMutation();
  const installMutation = useInstallDownloadableResourceMutation();
  const cancelMutation = useCancelDownloadableResourceMutation();
  const deleteMutation = useDeleteDownloadableResourceMutation();
  const [progress, setProgress] = useState<Record<string, DownloadableResourceInstallProgressDto>>(
    {},
  );
  const reportError = useCallback(
    (error: unknown) =>
      pushToast({ level: "error", title: t("resourceActionFailed"), message: formatError(error) }),
    [pushToast, t],
  );
  const retry = useCallback(() => void resourcesQuery.refetch(), [resourcesQuery]);
  const refresh = useCallback(
    () => refreshMutation.mutate(undefined, { onError: reportError }),
    [refreshMutation, reportError],
  );
  const install = useCallback(
    (resourceId: string) => {
      installMutation.mutate(
        {
          request: { resource_id: resourceId },
          onProgress: (next) =>
            setProgress((current) => ({ ...current, [next.resource_id]: next })),
        },
        { onError: reportError },
      );
    },
    [installMutation, reportError],
  );
  const cancel = useCallback(
    (resourceId: string) =>
      cancelMutation.mutate({ resource_id: resourceId }, { onError: reportError }),
    [cancelMutation, reportError],
  );
  const remove = useCallback(
    (resourceId: string) =>
      deleteMutation.mutate({ resource_id: resourceId }, { onError: reportError }),
    [deleteMutation, reportError],
  );

  if (resourcesQuery.isPending) {
    return (
      <AppPanel variant="section" className="h-full min-h-0 overflow-hidden">
        <LoadingPanel label={t("loadingResources")} />
      </AppPanel>
    );
  }
  if (resourcesQuery.isError || !resourcesQuery.data) {
    return (
      <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
        <SectionHeader title={t("resources")} />
        <div className="grid gap-3 p-3">
          <EmptyState
            title={t("resourcesUnavailable")}
            description={formatError(resourcesQuery.error)}
          />
          <AppButton variant="secondary" onClick={retry}>
            <RefreshCw aria-hidden="true" className="size-4" />
            {t("retryResources")}
          </AppButton>
        </div>
      </AppPanel>
    );
  }

  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
      <SectionHeader title={t("resources")} />
      <div className="grid gap-4 p-3">
        <div className="flex justify-end">
          <AppButton variant="secondary" disabled={refreshMutation.isPending} onClick={refresh}>
            <RefreshCw aria-hidden="true" className="size-4" />
            {t("refreshCatalog")}
          </AppButton>
        </div>
        {resourcesQuery.data.resources.map((resource) => (
          <ResourceCard
            key={resource.id}
            resource={resource}
            progress={progress[resource.id]}
            busy={
              installMutation.isPending &&
              installMutation.variables.request.resource_id === resource.id
            }
            onInstall={install}
            onCancel={cancel}
            onDelete={remove}
          />
        ))}
      </div>
    </AppPanel>
  );
}

function ResourceCard({
  resource,
  progress,
  busy,
  onInstall,
  onCancel,
  onDelete,
}: {
  resource: DownloadableResourceStatusDto;
  progress?: DownloadableResourceInstallProgressDto;
  busy: boolean;
  onInstall: (resourceId: string) => void;
  onCancel: (resourceId: string) => void;
  onDelete: (resourceId: string) => void;
}) {
  const { t } = useTranslation("settings");
  const downloaded = progress?.downloaded_bytes ?? resource.downloaded_bytes;
  const total = progress?.total_bytes ?? resource.size_bytes;
  const percent = total > 0 ? Math.min(100, (downloaded / total) * 100) : 0;
  const progressStyle = useMemo(() => ({ width: `${percent}%` }), [percent]);
  const downloading = busy || resource.state === "downloading" || resource.state === "verifying";
  const installed = resource.installed_version !== null;
  const install = useCallback(() => onInstall(resource.id), [onInstall, resource.id]);
  const cancel = useCallback(() => onCancel(resource.id), [onCancel, resource.id]);
  const remove = useCallback(() => onDelete(resource.id), [onDelete, resource.id]);
  return (
    <section className="grid gap-3 border border-app-border bg-black/15 p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-app-text">{resourceLabel(resource.id)}</h3>
          <p className="mt-1 text-xs text-app-muted">
            {t("resourceSummary", {
              state: t(`resourceStates.${resource.state}`),
              size: formatBytes(resource.size_bytes),
              version: resource.available_version,
            })}
          </p>
          {resource.message ? (
            <p className="mt-2 text-xs text-rose-200">{resource.message}</p>
          ) : null}
        </div>
        <div className="flex gap-2">
          {downloading ? (
            <AppButton variant="ghost" onClick={cancel}>
              <X aria-hidden="true" className="size-4" /> {t("cancelDownload")}
            </AppButton>
          ) : (
            <>
              {installed ? (
                <AppButton variant="danger" onClick={remove}>
                  <Trash2 aria-hidden="true" className="size-4" /> {t("deleteResource")}
                </AppButton>
              ) : null}
              {resource.state !== "ready" ? (
                <AppButton variant="secondary" onClick={install}>
                  <Download aria-hidden="true" className="size-4" />
                  {resource.state === "update_available"
                    ? t("updateResource")
                    : t("downloadResource")}
                </AppButton>
              ) : null}
            </>
          )}
        </div>
      </div>
      {downloading ? (
        <div className="grid gap-1">
          <div className="h-1.5 overflow-hidden bg-app-surface">
            <div className="h-full bg-brand-400" style={progressStyle} />
          </div>
          <p className="flex items-center gap-1 text-xs text-app-muted">
            <Loader2 aria-hidden="true" className="size-3 animate-spin" />
            {formatBytes(downloaded)} / {formatBytes(total)}
          </p>
        </div>
      ) : null}
    </section>
  );
}

function resourceLabel(id: string): string {
  const labels: Record<string, string> = {
    "lexicon-core": "Lexicon Core",
    "lexicon-semantic": "Lexicon Semantic Search",
    "anime-dbrating": "anime_dbrating",
    "wd-swinv2-tagger-v3": "WD SwinV2 Tagger v3",
  };
  return labels[id] ?? id;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}
