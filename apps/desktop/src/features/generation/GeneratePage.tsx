/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { useCallback, useRef, useState } from "react";

import type { CompiledGenerationPromptDto, RunHistoryStatusDto } from "../../types";
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
  useCachedActiveSubscriptionQuery,
  useClearGenerationDraftMutation,
  useCompilePromptMutation,
  useGenerationDraftQuery,
  useGenerationEstimateQuery,
  useGenerationSettingsQuery,
  usePauseGenerationMutation,
  usePromptPresetsQuery,
  useRefreshActiveSubscriptionMutation,
  useResourceImageQuery,
  useResumeGenerationMutation,
  useSaveGenerationDraftMutation,
  useStopGenerationMutation,
  useSubmitGenerationMutation,
} from "./data/useGenerationActions";
import { useGenerationGlobalSettingsQuery } from "./data/useGenerationGlobalSettingsQuery";
import { useGenerationStatusQuery, useRunHistoryQuery } from "./data/useGenerationStatusQuery";
import { formatGenerationError as formatError } from "./generation-page-utils";
import {
  buildSubmitGenerationBatchRequest,
  canSubmitGenerationDraft,
  createGenerationDraft,
  resetGenerationParameters,
} from "./model/generation-draft";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";
import { useGenerationPageActions } from "./state/useGenerationPageActions";
import { useQueueActionHandlers } from "./state/useQueueActionHandlers";

const EMPTY_ITEMS: [] = [];
const HISTORY_PAGE_LIMIT = 8;

export function GeneratePage() {
  const settingsQuery = useGenerationSettingsQuery();
  const globalSettingsQuery = useGenerationGlobalSettingsQuery();
  const storedDraftQuery = useGenerationDraftQuery();
  const saveDraftMutation = useSaveGenerationDraftMutation();
  const clearDraftMutation = useClearGenerationDraftMutation();
  const statusQuery = useGenerationStatusQuery();
  const accountQuery = useCachedActiveSubscriptionQuery();
  const refreshSubscriptionMutation = useRefreshActiveSubscriptionMutation();
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
    include_disabled: false,
    offset: 0,
    limit: 200,
  });
  const characterPresetsQuery = usePromptPresetsQuery({
    kind: "character",
    include_disabled: false,
    offset: 0,
    limit: 200,
  });
  const isOpus = accountQuery.data?.is_opus ?? false;
  const estimateQuery = useGenerationEstimateQuery(draft, isOpus);
  const activePreview = useGenerationEventStore((state) => state.activePreview);
  const filmstrip = useGenerationEventStore((state) => state.filmstrip);
  const selectPreview = useGenerationEventStore((state) => state.selectPreview);
  const lastError = useGenerationEventStore((state) => state.lastError);
  const selectedHistoryItemId = useGenerationEventStore((state) => state.selectedHistoryItemId);
  const selectHistoryItem = useGenerationEventStore((state) => state.selectHistoryItem);
  const promptPanelRef = useRef<GenerationPromptPanelHandle>(null);
  const [historyStatusFilter, setHistoryStatusFilter] = useState<"all" | RunHistoryStatusDto>(
    "all",
  );
  const [historyOffset, setHistoryOffset] = useState(0);
  const historyQuery = useRunHistoryQuery({
    offset: historyOffset,
    limit: HISTORY_PAGE_LIMIT,
    kind: "generation",
    status: historyStatusFilter === "all" ? null : historyStatusFilter,
  });
  const finalResource = activePreview?.kind === "resource" ? activePreview.resource : null;
  const finalImageQuery = useResourceImageQuery(finalResource);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [queueError, setQueueError] = useState<string | null>(null);
  const [compileDialogOpen, setCompileDialogOpen] = useState(false);
  const [compileError, setCompileError] = useState<string | null>(null);
  const [compiledPreview, setCompiledPreview] = useState<CompiledGenerationPromptDto | null>(null);

  const status = statusQuery.data;
  const historyItems = historyQuery.data?.items ?? EMPTY_ITEMS;
  const selectedHistoryItem =
    historyItems.find((item) => item.run_id === selectedHistoryItemId) ?? null;
  const generationActions = useGenerationPageActions({
    draft,
    activePreview,
    selectedHistoryItem,
    selectHistoryItem,
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

  const handleSubmit = useCallback(() => {
    if (!draft || submitMutation.isPending) {
      return;
    }
    if (!canSubmitGenerationDraft(draft)) {
      setValidationError("Positive prompt is required.");
      promptPanelRef.current?.focusPositive();
      return;
    }
    setValidationError(null);
    setSubmitError(null);
    flushDraft();
    void submitMutation
      .mutateAsync(buildSubmitGenerationBatchRequest(draft, undefined, { isOpus }))
      .catch((error) => setSubmitError(formatError(error)));
  }, [draft, flushDraft, isOpus, submitMutation]);

  const handleCompile = useCallback(() => {
    if (!draft) {
      return;
    }
    setCompileDialogOpen(true);
    setCompileError(null);
    setCompiledPreview(null);
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
      .then(setCompiledPreview)
      .catch((error) => setCompileError(formatError(error)));
  }, [compileMutation, draft]);

  const handleClearStoredDraft = useCallback(() => {
    if (!settingsQuery.data) {
      return;
    }
    void clearDraftMutation.mutateAsync().then(async () => {
      replaceDraft(createGenerationDraft(settingsQuery.data), { persist: "immediate" });
      await storedDraftQuery.refetch();
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
              refreshPending={refreshSubscriptionMutation.isPending}
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
              onRefreshBalance={() => refreshSubscriptionMutation.mutate()}
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
            preview={activePreview}
            finalImage={finalImageQuery.data}
            finalImagePending={finalImageQuery.isPending && Boolean(finalResource)}
            finalImageError={finalImageQuery.isError ? formatError(finalImageQuery.error) : null}
            status={status}
            statusError={statusQuery.isError ? formatError(statusQuery.error) : null}
            lastError={lastError?.message ?? null}
            filmstrip={filmstrip}
            savePending={generationActions.exportPending}
            handoffPending={generationActions.handoffPending}
            compilePending={compileMutation.isPending}
            queueControls={queueControls}
            onSelectPreview={selectPreview}
            onSavePreview={generationActions.handleSavePreview}
            onSendPreviewToDirector={generationActions.handleSendPreviewToDirector}
            onCompilePrompt={handleCompile}
          />
        }
        history={
          <GenerationHistoryRail
            items={historyItems}
            pending={historyQuery.isPending}
            error={historyQuery.isError ? formatError(historyQuery.error) : null}
            selectedItemId={selectedHistoryItemId}
            statusFilter={historyStatusFilter}
            offset={historyQuery.data?.offset ?? historyOffset}
            limit={historyQuery.data?.limit ?? HISTORY_PAGE_LIMIT}
            total={historyQuery.data?.total ?? 0}
            rerunPending={generationActions.rerunPending}
            deletePending={generationActions.deletePending}
            exportPending={generationActions.exportPending}
            handoffPending={generationActions.handoffPending}
            onSelect={selectHistoryItem}
            onStatusFilterChange={(next) => {
              setHistoryStatusFilter(next);
              setHistoryOffset(0);
            }}
            onPreviousPage={() =>
              setHistoryOffset((value) => Math.max(0, value - HISTORY_PAGE_LIMIT))
            }
            onNextPage={() => setHistoryOffset((value) => value + HISTORY_PAGE_LIMIT)}
            onRerunSelected={generationActions.handleRerunSelected}
            onDeleteSelected={generationActions.handleDeleteSelected}
            onExportSelected={generationActions.handleExportSelected}
            onSendSelectedToDirector={generationActions.handleSendSelectedToDirector}
          />
        }
      />

      {queueError || generationActions.error ? (
        <div className="pointer-events-none absolute top-3 left-1/2 z-40 -translate-x-1/2 border border-rose-500/40 bg-rose-950/90 px-3 py-2 text-sm text-rose-100 shadow-app-panel">
          {queueError ?? generationActions.error}
        </div>
      ) : null}

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
