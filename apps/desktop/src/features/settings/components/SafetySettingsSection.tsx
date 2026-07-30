import { Download, Loader2, RotateCw, Trash2, X } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppPanel, EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type {
  GlobalSettingsDto,
  ImageAnalysisModelIdDto,
  ImageAnalysisModelInstallProgressDto,
  ImageAnalysisModelStatusDto,
} from "@/types";

import {
  useCancelImageAnalysisModelInstallMutation,
  useDeleteImageAnalysisModelMutation,
  useImageAnalysisModelsQuery,
  useInstallImageAnalysisModelMutation,
} from "../data/useImageAnalysisModels";
import { formatError } from "../settings-utils";
import { CheckboxField, LoadingPanel, SectionHeader } from "./SettingsControls";

type Props = {
  draft: GlobalSettingsDto;
  updateDraft: (draft: GlobalSettingsDto) => void;
};

export function SafetySettingsSection({ draft, updateDraft }: Props) {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const modelsQuery = useImageAnalysisModelsQuery();
  const installMutation = useInstallImageAnalysisModelMutation();
  const cancelMutation = useCancelImageAnalysisModelInstallMutation();
  const deleteMutation = useDeleteImageAnalysisModelMutation();
  const [progress, setProgress] = useState<
    Partial<Record<ImageAnalysisModelIdDto, ImageAnalysisModelInstallProgressDto>>
  >({});
  const [deleteOpen, setDeleteOpen] = useState(false);
  const models = useMemo(
    () => new Map(modelsQuery.data?.map((model) => [model.model_id, model]) ?? []),
    [modelsQuery.data],
  );
  const primary = models.get("anime_db_rating");
  const wd = models.get("wd_swinv2_tagger_v3");

  const reportError = useCallback(
    (error: unknown) =>
      pushToast({
        level: "error",
        title: t("modelActionFailed"),
        message: formatError(error),
      }),
    [pushToast, t],
  );
  const install = useCallback(
    (modelId: ImageAnalysisModelIdDto) => {
      installMutation.mutate(
        {
          request: { model_id: modelId },
          onProgress: (next) => setProgress((current) => ({ ...current, [next.model_id]: next })),
        },
        { onError: reportError },
      );
    },
    [installMutation, reportError],
  );
  const cancel = useCallback(
    (modelId: ImageAnalysisModelIdDto) => {
      cancelMutation.mutate({ model_id: modelId }, { onError: reportError });
    },
    [cancelMutation, reportError],
  );
  const updateAutoReview = useCallback(
    (enabled: boolean) => {
      updateDraft({
        ...draft,
        safety: { ...draft.safety, wd_auto_review_enabled: enabled },
      });
    },
    [draft, updateDraft],
  );
  const deleteWd = useCallback(() => {
    deleteMutation.mutate(
      { model_id: "wd_swinv2_tagger_v3" },
      {
        onSuccess: () => setDeleteOpen(false),
        onError: reportError,
      },
    );
  }, [deleteMutation, reportError]);
  const installPrimary = useCallback(() => install("anime_db_rating"), [install]);
  const cancelPrimary = useCallback(() => cancel("anime_db_rating"), [cancel]);
  const installWd = useCallback(() => install("wd_swinv2_tagger_v3"), [install]);
  const cancelWd = useCallback(() => cancel("wd_swinv2_tagger_v3"), [cancel]);
  const openDeleteWd = useCallback(() => setDeleteOpen(true), []);
  const closeDeleteWd = useCallback(() => setDeleteOpen(false), []);
  const retryModels = useCallback(() => void modelsQuery.refetch(), [modelsQuery]);

  if (modelsQuery.isPending) {
    return (
      <AppPanel variant="section" className="h-full min-h-0 overflow-hidden">
        <LoadingPanel label={t("loadingSafetyModels")} />
      </AppPanel>
    );
  }

  if (modelsQuery.isError) {
    return (
      <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
        <SectionHeader title={t("safety")} />
        <div className="grid gap-3 p-3">
          <EmptyState
            title={t("safetyModelsUnavailable")}
            description={formatError(modelsQuery.error)}
          />
          <AppButton variant="secondary" onClick={retryModels}>
            <RotateCw aria-hidden="true" className="size-4" />
            {t("retrySafetyModels")}
          </AppButton>
        </div>
      </AppPanel>
    );
  }

  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-y-auto">
      <SectionHeader title={t("safety")} />
      <div className="grid gap-4 p-3">
        <p className="text-sm leading-5 text-app-muted">{t("safetyDescriptionLong")}</p>
        <ModelCard
          model={primary}
          progress={progress.anime_db_rating}
          installing={
            installMutation.isPending &&
            installMutation.variables?.request.model_id === "anime_db_rating"
          }
          title={t("dbratingModel")}
          description={t("dbratingDescription")}
          onInstall={installPrimary}
          onCancel={cancelPrimary}
        />
        <ModelCard
          model={wd}
          progress={progress.wd_swinv2_tagger_v3}
          installing={
            installMutation.isPending &&
            installMutation.variables?.request.model_id === "wd_swinv2_tagger_v3"
          }
          title={t("wdModel")}
          description={t("wdDescription")}
          optional
          onInstall={installWd}
          onCancel={cancelWd}
          onDelete={openDeleteWd}
          deleteDisabled={draft.safety.wd_auto_review_enabled}
        />
        <CheckboxField
          label={t("wdAutoReview")}
          checked={draft.safety.wd_auto_review_enabled}
          disabled={wd?.state !== "ready"}
          onChange={updateAutoReview}
        />
        <p className="text-xs leading-4 text-app-muted">{t("wdAutoReviewDescription")}</p>
      </div>
      <AppModal open={deleteOpen} title={t("deleteWdTitle")} onClose={closeDeleteWd}>
        <div className="grid gap-4">
          <p className="text-sm text-app-muted">{t("deleteWdDescription")}</p>
          <div className="flex justify-end gap-2">
            <AppButton variant="ghost" onClick={closeDeleteWd}>
              {t("cancel")}
            </AppButton>
            <AppButton variant="danger" disabled={deleteMutation.isPending} onClick={deleteWd}>
              <Trash2 aria-hidden="true" className="size-4" />
              {t("deleteModel")}
            </AppButton>
          </div>
        </div>
      </AppModal>
    </AppPanel>
  );
}

function ModelCard({
  model,
  progress,
  installing = false,
  title,
  description,
  optional = false,
  onInstall,
  onCancel,
  onDelete,
  deleteDisabled = false,
}: {
  model?: ImageAnalysisModelStatusDto;
  progress?: ImageAnalysisModelInstallProgressDto;
  installing?: boolean;
  title: string;
  description: string;
  optional?: boolean;
  onInstall: () => void;
  onCancel: () => void;
  onDelete?: () => void;
  deleteDisabled?: boolean;
}) {
  const { t } = useTranslation("settings");
  const downloaded = progress?.downloaded_bytes ?? model?.downloaded_bytes ?? 0;
  const total = progress?.total_bytes ?? model?.size_bytes ?? 0;
  const percent = total > 0 ? Math.min(100, (downloaded / total) * 100) : 0;
  const progressStyle = useMemo(() => ({ width: `${percent}%` }), [percent]);
  const state = installing ? "installing" : (model?.state ?? "failed");

  return (
    <section className="grid gap-3 border border-app-border bg-black/15 p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-app-text">{title}</h3>
            <span className="text-[10px] font-semibold tracking-wide text-app-muted uppercase">
              {optional ? t("optionalModel") : t("requiredModel")}
            </span>
          </div>
          <p className="mt-1 text-xs leading-4 text-app-muted">{description}</p>
          {model ? (
            <p className="mt-2 text-xs text-app-muted">
              {t(`modelStates.${model.state}`)} · {formatBytes(model.size_bytes)} ·{" "}
              {model.revision.slice(0, 12)}
            </p>
          ) : null}
        </div>
        <div className="flex shrink-0 gap-2">
          {state === "installing" ? (
            <AppButton variant="ghost" onClick={onCancel}>
              <X aria-hidden="true" className="size-4" />
              {t("cancelDownload")}
            </AppButton>
          ) : state === "ready" ? (
            onDelete ? (
              <AppButton variant="danger" disabled={deleteDisabled} onClick={onDelete}>
                <Trash2 aria-hidden="true" className="size-4" />
                {t("deleteModel")}
              </AppButton>
            ) : null
          ) : (
            <AppButton variant="secondary" onClick={onInstall}>
              {state === "missing" ? (
                <Download aria-hidden="true" className="size-4" />
              ) : (
                <RotateCw aria-hidden="true" className="size-4" />
              )}
              {state === "missing" ? t("downloadModel") : t("retryDownload")}
            </AppButton>
          )}
        </div>
      </div>
      {state === "installing" ? (
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
      {model?.message ? <p className="text-xs text-rose-200">{model.message}</p> : null}
    </section>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}
