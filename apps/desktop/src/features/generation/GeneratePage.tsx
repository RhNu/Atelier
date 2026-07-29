/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { describeError, frontendLogger } from "@/app/logger";
import { useActiveAccountSummaryQuery } from "@/features/account/data/useActiveAccountSummaryQuery";
import { useToastStore } from "@/stores/toast-store";
import type { CompiledGenerationPromptDto, GenerationBatchHistoryStatusDto } from "@/types";

import { AdvancedGenerationInputs } from "./components/AdvancedGenerationInputs";
import {
  GenerationLoadingState,
  GenerationSettingsError,
  QueueControls,
} from "./components/GeneratePageSupport";
import { GenerationActionDock } from "./components/GenerationActionDock";
import { GenerationHistoryRail } from "./components/GenerationHistoryRail";
import { GenerationParamsPanel } from "./components/GenerationParamsPanel";
import { GenerationPreviewStage } from "./components/GenerationPreviewStage";
import { GenerationPromptCompileDialog } from "./components/GenerationPromptCompileDialog";
import {
  GenerationPromptPanel,
  type GenerationPromptPanelHandle,
} from "./components/GenerationPromptPanel";
import { GenerationWorkbenchLayout } from "./components/GenerationWorkbenchLayout";
import {
  useClearGenerationDraftMutation,
  useCompilePromptMutation,
  useGenerationDraftQuery,
  useGenerationEstimateQuery,
  useGenerationSettingsQuery,
  usePauseGenerationMutation,
  usePromptPresetsQuery,
  useResumeGenerationMutation,
  useSaveGenerationDraftMutation,
  useStopGenerationMutation,
  useSubmitGenerationMutation,
} from "./data/useGenerationActions";
import { useGenerationGlobalSettingsQuery } from "./data/useGenerationGlobalSettingsQuery";
import {
  useGenerationHistoryBatchQuery,
  useGenerationHistoryQuery,
  useGenerationStatusQuery,
} from "./data/useGenerationStatusQuery";
import { formatGenerationError as formatError } from "./generation-page-utils";
import {
  buildSubmitGenerationBatchRequest,
  canSubmitGenerationDraft,
  createGenerationDraft,
  resetGenerationParameters,
} from "./model/generation-draft";
import { buildGenerationBatchView, selectDefaultRequest } from "./model/generation-preview-model";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";
import { useGenerationPageActions } from "./state/useGenerationPageActions";
import { useQueueActionHandlers } from "./state/useQueueActionHandlers";

const EMPTY_ITEMS: [] = [];
const HISTORY_PAGE_LIMIT = 8;

export function GeneratePage() {
  const { t } = useTranslation("generation");
  const pushToast = useToastStore((state) => state.push);
  const settingsQuery = useGenerationSettingsQuery();
  const globalSettingsQuery = useGenerationGlobalSettingsQuery();
  const storedDraftQuery = useGenerationDraftQuery();
  const saveDraftMutation = useSaveGenerationDraftMutation();
  const clearDraftMutation = useClearGenerationDraftMutation();
  const statusQuery = useGenerationStatusQuery();
  const accountQuery = useActiveAccountSummaryQuery();
  const submitMutation = useSubmitGenerationMutation();
  const pauseMutation = usePauseGenerationMutation();
  const resumeMutation = useResumeGenerationMutation();
  const stopMutation = useStopGenerationMutation();
  const compileMutation = useCompilePromptMutation();
  const saveDraft = useCallback(
    (draft: Parameters<typeof saveDraftMutation.mutateAsync>[0]) =>
      saveDraftMutation.mutateAsync(draft),
    [saveDraftMutation],
  );
  const {
    draft,
    patchDraft,
    patchSize,
    replaceDraft,
    flushDraft,
    retrySave,
    saveError: draftSaveError,
  } = useGenerationDraft({
    settings: settingsQuery.data,
    storedDraft: storedDraftQuery.isError ? null : storedDraftQuery.data,
    sourceReady:
      !settingsQuery.isPending && (!storedDraftQuery.isPending || storedDraftQuery.isError),
    saveDraft,
  });
  const mainPresetsQuery = usePromptPresetsQuery({
    kind: "main",
    offset: 0,
    limit: 200,
  });
  const characterPresetsQuery = usePromptPresetsQuery({
    kind: "character",
    offset: 0,
    limit: 200,
  });
  const isOpus = accountQuery.data?.is_opus ?? false;
  const estimateQuery = useGenerationEstimateQuery(draft, isOpus);
  const liveBatchId = useGenerationEventStore((state) => state.liveBatchId);
  const storedViewBatchId = useGenerationEventStore((state) => state.viewBatchId);
  const latestJobId = useGenerationEventStore((state) => state.latestJobId);
  const selectedJobId = useGenerationEventStore((state) => state.selectedJobId);
  const focusedSampleIndex = useGenerationEventStore((state) => state.focusedSampleIndex);
  const focusMode = useGenerationEventStore((state) => state.focusMode);
  const previews = useGenerationEventStore((state) => state.previews);
  const lastError = useGenerationEventStore((state) => state.lastError);
  const syncActiveBatch = useGenerationEventStore((state) => state.syncActiveBatch);
  const selectBatch = useGenerationEventStore((state) => state.selectBatch);
  const selectRequest = useGenerationEventStore((state) => state.selectRequest);
  const focusSample = useGenerationEventStore((state) => state.focusSample);
  const showRequestGrid = useGenerationEventStore((state) => state.showRequestGrid);
  const resumeFollow = useGenerationEventStore((state) => state.resumeFollow);
  const promptPanelRef = useRef<GenerationPromptPanelHandle>(null);
  const [historyStatusFilter, setHistoryStatusFilter] = useState<
    "all" | GenerationBatchHistoryStatusDto
  >("all");
  const [historyOffset, setHistoryOffset] = useState(0);
  const historyQuery = useGenerationHistoryQuery({
    offset: historyOffset,
    limit: HISTORY_PAGE_LIMIT,
    status: historyStatusFilter === "all" ? null : historyStatusFilter,
  });
  const [validationError, setValidationError] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [queueError, setQueueError] = useState<string | null>(null);
  const [compileDialogOpen, setCompileDialogOpen] = useState(false);
  const [compileError, setCompileError] = useState<string | null>(null);
  const [compiledPreview, setCompiledPreview] = useState<CompiledGenerationPromptDto | null>(null);

  const status = statusQuery.data;
  const effectiveLiveBatchId = liveBatchId ?? status?.batch_id ?? null;
  const viewBatchId = storedViewBatchId ?? effectiveLiveBatchId;
  const detailQuery = useGenerationHistoryBatchQuery(viewBatchId);
  const batchView = useMemo(
    () =>
      buildGenerationBatchView({
        batchId: viewBatchId,
        detail: detailQuery.data,
        status,
        previews,
      }),
    [detailQuery.data, previews, status, viewBatchId],
  );
  const selectedRequest = useMemo(
    () =>
      selectDefaultRequest(batchView, selectedJobId, latestJobId ?? status?.current_job_id ?? null),
    [batchView, latestJobId, selectedJobId, status?.current_job_id],
  );
  const selectedSample =
    focusedSampleIndex === null
      ? null
      : (selectedRequest?.samples.find((sample) => sample.sampleIndex === focusedSampleIndex) ??
        null);
  const historyBatches = historyQuery.data?.items ?? EMPTY_ITEMS;
  const isViewingLive = Boolean(viewBatchId && viewBatchId === effectiveLiveBatchId);

  useEffect(() => {
    syncActiveBatch(status?.batch_id ?? null, status?.current_job_id ?? null);
  }, [status?.batch_id, status?.current_job_id, syncActiveBatch]);

  const generationActions = useGenerationPageActions({
    draft,
    batch: batchView,
    selectedRequest,
    selectedSample,
    onBatchDeleted: resumeFollow,
    onRequestDeleted: showRequestGrid,
  });
  const batchStatus = status?.batch_status ?? null;
  const canPause = batchStatus === "running" || batchStatus === "waiting";
  const canResume = batchStatus === "paused";
  const canStop =
    batchStatus === "running" || batchStatus === "waiting" || batchStatus === "paused";
  const { handlePause, handleResume, handleStop } = useQueueActionHandlers({
    pause: pauseMutation.mutateAsync,
    resume: resumeMutation.mutateAsync,
    stop: stopMutation.mutateAsync,
    setQueueError,
  });
  const interactionError = queueError ?? generationActions.error;
  useEffect(() => {
    if (interactionError) {
      pushToast({ level: "error", title: t("generationActionFailed"), message: interactionError });
    }
  }, [interactionError, pushToast, t]);

  const handleSubmit = useCallback(() => {
    if (!draft || submitMutation.isPending) {
      return;
    }
    if (!canSubmitGenerationDraft(draft)) {
      setValidationError(t("positivePromptRequired"));
      promptPanelRef.current?.focusPositive();
      return;
    }
    setValidationError(null);
    setSubmitError(null);
    flushDraft();
    frontendLogger.info("Generation batch submission started");
    void submitMutation
      .mutateAsync(buildSubmitGenerationBatchRequest(draft, undefined, { isOpus }))
      .then(() => {
        frontendLogger.info("Generation batch submission completed");
      })
      .catch((error: unknown) => {
        frontendLogger.error("Generation batch submission failed", {
          error: describeError(error),
        });
        setSubmitError(formatError(error));
      });
  }, [draft, flushDraft, isOpus, submitMutation, t]);

  const handleCompile = useCallback(() => {
    if (!draft) {
      return;
    }
    setCompileDialogOpen(true);
    setCompileError(null);
    setCompiledPreview(null);
    frontendLogger.info("Generation prompt compilation started");
    void compileMutation
      .mutateAsync({
        prompt: draft.prompt,
        main_preset_id: draft.mainPresetId,
        negative_prompt: draft.negativePrompt.trim() ? draft.negativePrompt : null,
        characters: draft.characters
          .filter(
            (character) =>
              character.enabled && (character.prompt.trim() || Boolean(character.presetId)),
          )
          .map((character) => ({
            preset_id: character.presetId,
            prompt: character.prompt.trim(),
            negative_prompt: character.negativePrompt.trim() || null,
            enabled: character.enabled,
          })),
        max_depth: 8,
      })
      .then((preview) => {
        frontendLogger.info("Generation prompt compilation completed");
        setCompiledPreview(preview);
      })
      .catch((error: unknown) => {
        frontendLogger.error("Generation prompt compilation failed", {
          error: describeError(error),
        });
        setCompileError(formatError(error));
      });
  }, [compileMutation, draft]);

  const handleClearStoredDraft = useCallback(() => {
    if (!settingsQuery.data) {
      return;
    }
    frontendLogger.info("Generation draft reset started");
    void clearDraftMutation
      .mutateAsync()
      .then(async () => {
        replaceDraft(createGenerationDraft(settingsQuery.data), { persist: "immediate" });
        await storedDraftQuery.refetch();
        frontendLogger.info("Generation draft reset completed");
      })
      .catch((error: unknown) => {
        frontendLogger.error("Generation draft reset failed", {
          error: describeError(error),
        });
        setSubmitError(formatError(error));
      });
  }, [clearDraftMutation, replaceDraft, settingsQuery.data, storedDraftQuery]);

  if (settingsQuery.isError) {
    return <GenerationSettingsError error={settingsQuery.error} />;
  }
  if (settingsQuery.isPending || !draft) {
    return <GenerationLoadingState />;
  }

  const queueControls = (
    <QueueControls
      canPause={canPause}
      canResume={canResume}
      canStop={canStop}
      pausePending={pauseMutation.isPending}
      resumePending={resumeMutation.isPending}
      stopPending={stopMutation.isPending}
      onPause={handlePause}
      onResume={handleResume}
      onStop={handleStop}
    />
  );

  return (
    <div className="relative h-full min-h-0">
      <GenerationWorkbenchLayout
        sidebar={
          <>
            <div
              data-testid="generation-settings-scroll"
              className="min-h-0 flex-1 overflow-y-auto overscroll-contain"
            >
              <GenerationPromptPanel
                ref={promptPanelRef}
                draft={draft}
                mainPresets={mainPresetsQuery.data?.items ?? EMPTY_ITEMS}
                mainPresetsPending={mainPresetsQuery.isPending}
                onPatch={patchDraft}
                onFlush={flushDraft}
              />
              <AdvancedGenerationInputs
                draft={draft}
                onPatch={patchDraft}
                onFlush={flushDraft}
                characterPresets={characterPresetsQuery.data?.items ?? EMPTY_ITEMS}
                vibeImportPending={generationActions.vibeImportPending}
                vibeExportPending={generationActions.vibeExportPending}
                imageImportPending={generationActions.imageImportPending}
                vibeEnsurePending={generationActions.vibeEnsurePending}
                onPickImageResources={generationActions.handlePickImageResources}
                onPickVibeEncoding={generationActions.handlePickVibeEncoding}
                onReleaseImageResources={generationActions.handleReleaseImageResources}
                onImportVibeDocuments={generationActions.handleImportVibeDocuments}
                onExportVibeDocument={generationActions.handleExportVibeDocument}
                developerMode={globalSettingsQuery.data?.frontend.developer_mode === true}
              />
              <GenerationParamsPanel
                draft={draft}
                onPatch={patchDraft}
                onPatchSize={patchSize}
                onFlush={flushDraft}
              />
            </div>
            <GenerationActionDock
              draft={draft}
              balance={accountQuery.data?.anlas_balance ?? null}
              balancePending={accountQuery.isPending}
              balanceError={accountQuery.isError ? formatError(accountQuery.error) : null}
              estimate={estimateQuery.data?.total_cost ?? null}
              estimatePending={estimateQuery.isPending}
              estimateError={estimateQuery.isError ? formatError(estimateQuery.error) : null}
              submitPending={submitMutation.isPending}
              validationError={validationError}
              submitError={submitError}
              draftLoadError={storedDraftQuery.isError ? formatError(storedDraftQuery.error) : null}
              draftSaveError={draftSaveError}
              onPatch={patchDraft}
              onFlush={flushDraft}
              onSubmit={handleSubmit}
              onResetParameters={() =>
                replaceDraft(resetGenerationParameters(draft, settingsQuery.data), {
                  persist: "immediate",
                })
              }
              onRetryDraftSave={retrySave}
              onClearStoredDraft={handleClearStoredDraft}
            />
          </>
        }
        preview={
          <GenerationPreviewStage
            batch={batchView}
            selectedRequest={selectedRequest}
            focusedSampleIndex={focusedSampleIndex}
            focusMode={focusMode}
            isViewingLive={isViewingLive}
            liveBatchAvailable={Boolean(effectiveLiveBatchId)}
            statusError={statusQuery.isError ? formatError(statusQuery.error) : null}
            lastError={lastError?.message ?? null}
            savePending={generationActions.exportPending}
            zipPending={generationActions.zipPending}
            handoffPending={generationActions.handoffPending}
            rerunPending={generationActions.rerunPending}
            deletePending={generationActions.deletePending}
            compilePending={compileMutation.isPending}
            queueControls={queueControls}
            onSelectRequest={selectRequest}
            onFocusSample={focusSample}
            onShowRequestGrid={showRequestGrid}
            onResumeLive={resumeFollow}
            onSaveSample={generationActions.handleSaveSample}
            onSendSampleToDirector={generationActions.handleSendSampleToDirector}
            onExportRequest={generationActions.handleExportRequest}
            onRerunRequest={generationActions.handleRerunRequest}
            onDeleteRequest={generationActions.handleDeleteRequest}
            onCompilePrompt={handleCompile}
          />
        }
        history={
          <GenerationHistoryRail
            batches={historyBatches}
            pending={historyQuery.isPending}
            error={historyQuery.isError ? formatError(historyQuery.error) : null}
            selectedBatchId={viewBatchId}
            statusFilter={historyStatusFilter}
            offset={historyQuery.data?.offset ?? historyOffset}
            limit={historyQuery.data?.limit ?? HISTORY_PAGE_LIMIT}
            total={historyQuery.data?.total ?? 0}
            rerunPending={generationActions.rerunPending}
            deletePending={generationActions.deletePending}
            exportPending={generationActions.zipPending}
            onSelect={selectBatch}
            onStatusFilterChange={(next) => {
              setHistoryStatusFilter(next);
              setHistoryOffset(0);
            }}
            onPreviousPage={() =>
              setHistoryOffset((value) => Math.max(0, value - HISTORY_PAGE_LIMIT))
            }
            onNextPage={() => setHistoryOffset((value) => value + HISTORY_PAGE_LIMIT)}
            onRerunSelected={generationActions.handleRerunBatch}
            onDeleteSelected={generationActions.handleDeleteBatch}
            onExportSelected={generationActions.handleExportBatch}
          />
        }
      />

      <GenerationPromptCompileDialog
        open={compileDialogOpen}
        pending={compileMutation.isPending}
        error={compileError}
        compiled={compiledPreview}
        onClose={() => setCompileDialogOpen(false)}
      />
    </div>
  );
}
