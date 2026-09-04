/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { describeError, frontendLogger } from "@/app/logger";
import { resolveOpusAllowance } from "@/features/account/components/opus-allowance";
import { useActiveAccountSummaryQuery } from "@/features/account/data/useActiveAccountSummaryQuery";
import { PromptEditorSettingsProvider } from "@/features/prompt-editor";
import { useToastStore } from "@/stores/toast-store";
import type {
  CompiledGenerationPromptDto,
  GenerationBatchHistoryStatusDto,
  ResourceRefDto,
} from "@/types";

import { AdvancedGenerationInputs } from "./components/AdvancedGenerationInputs";
import { CharacterPositionWorkspace } from "./components/CharacterPositionWorkspace";
import {
  GenerationLoadingState,
  GenerationSettingsError,
  QueueControls,
} from "./components/GeneratePageSupport";
import { GenerationActionDock } from "./components/GenerationActionDock";
import { GenerationHistoryDeleteConfirmation } from "./components/GenerationHistoryDeleteConfirmation";
import { GenerationHistoryRail } from "./components/GenerationHistoryRail";
import { GenerationParamsPanel } from "./components/GenerationParamsPanel";
import { GenerationPreviewStage } from "./components/GenerationPreviewStage";
import { GenerationPromptCompileDialog } from "./components/GenerationPromptCompileDialog";
import {
  GenerationPromptPanel,
  type GenerationPromptPanelHandle,
} from "./components/GenerationPromptPanel";
import { GenerationWorkbenchLayout } from "./components/GenerationWorkbenchLayout";
import { InpaintCanvasWorkspace } from "./components/InpaintCanvasWorkspace";
import {
  useClearGenerationDraftMutation,
  useCommitGenerationCanvasMutation,
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
import { useGenerationPromptTokenCounts } from "./data/useGenerationPromptTokenCounts";
import {
  useGenerationHistoryBatchQuery,
  useGenerationHistoryQuery,
  useGenerationStatusQuery,
} from "./data/useGenerationStatusQuery";
import { findModelDescriptor, useImageModelCatalog } from "./data/useImageModelCatalog";
import { formatGenerationError as formatError } from "./generation-page-utils";
import {
  buildSubmitGenerationBatchRequest,
  canSubmitGenerationDraft,
  createGenerationDraft,
  resetGenerationParameters,
  switchGenerationModel,
} from "./model/generation-draft";
import type { GenerationCharacterDraft, GenerationDraft } from "./model/generation-draft";
import { buildGenerationBatchView, selectDefaultRequest } from "./model/generation-preview-model";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";
import { useGenerationPageActions } from "./state/useGenerationPageActions";
import { useQueueActionHandlers } from "./state/useQueueActionHandlers";

const EMPTY_ITEMS: [] = [];
const HISTORY_PAGE_LIMIT = 8;
const PRESET_LIBRARY_LIMIT = 10_000;

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
  const modelCatalogQuery = useImageModelCatalog();
  const submitMutation = useSubmitGenerationMutation();
  const pauseMutation = usePauseGenerationMutation();
  const resumeMutation = useResumeGenerationMutation();
  const stopMutation = useStopGenerationMutation();
  const compileMutation = useCompilePromptMutation();
  const canvasCommitMutation = useCommitGenerationCanvasMutation();
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
  const promptTokenCounts = useGenerationPromptTokenCounts(draft);
  const mainPresetsQuery = usePromptPresetsQuery({
    kind: "main",
    model: draft?.model ?? null,
    offset: 0,
    limit: PRESET_LIBRARY_LIMIT,
  });
  const characterPresetsQuery = usePromptPresetsQuery({
    kind: "character",
    model: draft?.model ?? null,
    offset: 0,
    limit: PRESET_LIBRARY_LIMIT,
  });
  const subscription = accountQuery.data;
  const capabilities = draft
    ? findModelDescriptor(modelCatalogQuery.data, draft.model)?.capabilities
    : undefined;
  const estimateQuery = useGenerationEstimateQuery(draft, subscription, capabilities);
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
  const [selectedHistoryBatchIds, setSelectedHistoryBatchIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [historyDeleteConfirmationOpen, setHistoryDeleteConfirmationOpen] = useState(false);
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
  const [centerWorkspace, setCenterWorkspace] = useState<"preview" | "positions" | "inpaint">(
    "preview",
  );

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
  const positionCharacters = useMemo(
    () =>
      draft?.characters.filter(
        (character) =>
          character.enabled && (character.prompt.trim() || Boolean(character.presetId)),
      ) ?? [],
    [draft?.characters],
  );
  const canEditPositions = Boolean(
    capabilities?.character_position_mode &&
    positionCharacters.length >= (capabilities.can_position_one_character ? 1 : 2),
  );
  const historyBatches = historyQuery.data?.items ?? EMPTY_ITEMS;
  const isViewingLive = Boolean(viewBatchId && viewBatchId === effectiveLiveBatchId);

  useEffect(() => {
    syncActiveBatch(status?.batch_id ?? null, status?.current_job_id ?? null);
  }, [status?.batch_id, status?.current_job_id, syncActiveBatch]);

  const handleBatchHistoryDeleted = useCallback(() => {
    setSelectedHistoryBatchIds(new Set());
    setHistoryDeleteConfirmationOpen(false);
    resumeFollow();
  }, [resumeFollow]);
  const generationActions = useGenerationPageActions({
    draft,
    batch: batchView,
    selectedRequest,
    selectedSample,
    onBatchDeleted: handleBatchHistoryDeleted,
    onRequestDeleted: showRequestGrid,
  });
  const handleToggleHistoryBatch = useCallback((batchId: string) => {
    setSelectedHistoryBatchIds((current) => {
      const next = new Set(current);
      if (next.has(batchId)) next.delete(batchId);
      else next.add(batchId);
      return next;
    });
  }, []);
  const handleSelectAllHistoryBatches = useCallback(() => {
    setSelectedHistoryBatchIds(new Set(historyBatches.map((batch) => batch.batch_id)));
  }, [historyBatches]);
  const handleClearHistorySelection = useCallback(() => setSelectedHistoryBatchIds(new Set()), []);
  const handleDeleteSelectedHistoryBatches = useCallback(() => {
    if (selectedHistoryBatchIds.size > 0) setHistoryDeleteConfirmationOpen(true);
  }, [selectedHistoryBatchIds.size]);
  const handleConfirmDeleteSelectedHistoryBatches = useCallback(() => {
    generationActions.handleDeleteBatches([...selectedHistoryBatchIds]);
  }, [generationActions, selectedHistoryBatchIds]);
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
      pushToast({
        level: "error",
        title: t("generationActionFailed"),
        message: t("generationActionFailedDescription"),
      });
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
      .mutateAsync(
        buildSubmitGenerationBatchRequest(draft, undefined, { subscription, capabilities }),
      )
      .then(() => {
        frontendLogger.info("Generation batch submission completed");
      })
      .catch((error: unknown) => {
        frontendLogger.error("Generation batch submission failed", {
          error: describeError(error),
        });
        setSubmitError(formatError(error));
      });
  }, [capabilities, draft, flushDraft, subscription, submitMutation, t]);

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
        model: draft.model,
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

  const handleModelChange = useCallback(
    (model: Parameters<typeof findModelDescriptor>[1]) => {
      if (!draft) return;
      const descriptor = findModelDescriptor(modelCatalogQuery.data, model);
      if (!descriptor) return;
      replaceDraft(switchGenerationModel(draft, model, descriptor.capabilities), {
        persist: "immediate",
      });
    },
    [draft, modelCatalogQuery.data, replaceDraft],
  );

  const openPositionEditor = useCallback(() => setCenterWorkspace("positions"), []);
  const closePositionEditor = useCallback(() => setCenterWorkspace("preview"), []);
  const openInpaintEditor = useCallback(() => setCenterWorkspace("inpaint"), []);
  const applyCharacterPositions = useCallback(
    (positioned: GenerationCharacterDraft[]) => {
      if (!draft) return;
      const byId = new Map(positioned.map((character) => [character.id, character.position]));
      patchDraft(
        {
          characters: draft.characters.map((character) => ({
            ...character,
            position: byId.get(character.id) ?? character.position,
          })),
          characterPositionMode: "manual",
        },
        { persist: "immediate" },
      );
      setCenterWorkspace("preview");
    },
    [draft, patchDraft],
  );
  const applyInpaintCanvas = useCallback(
    (
      image: ResourceRefDto,
      inpaint: NonNullable<NonNullable<GenerationDraft["i2i"]>["inpaint"]>,
      size: GenerationDraft["size"],
    ) => {
      if (!draft?.i2i) return;
      const previous = [draft.i2i.image, draft.i2i.inpaint?.regionToReplace ?? null];
      patchDraft({ i2i: { ...draft.i2i, image, inpaint }, size }, { persist: "immediate" });
      setCenterWorkspace("preview");
      void generationActions.handleReleaseImageResources(previous);
    },
    [draft, generationActions, patchDraft],
  );
  const editSelectedOutputAsInpaint = useCallback(() => {
    if (!draft || !selectedSample?.resource || draft.model === "nai-diffusion-5-curated") return;
    patchDraft(
      {
        i2i: {
          image: selectedSample.resource,
          inpaint: null,
          strength: draft.i2i?.strength ?? 0.7,
          noise: draft.i2i?.noise ?? 0,
        },
      },
      { persist: "immediate" },
    );
    setCenterWorkspace("inpaint");
  }, [draft, patchDraft, selectedSample?.resource]);

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
              <PromptEditorSettingsProvider
                convertFullWidthPunctuation={
                  globalSettingsQuery.data?.frontend.convert_full_width_punctuation === true
                }
              >
                <GenerationPromptPanel
                  ref={promptPanelRef}
                  draft={draft}
                  mainPresets={mainPresetsQuery.data?.items ?? EMPTY_ITEMS}
                  mainPresetsPending={mainPresetsQuery.isPending}
                  onPatch={patchDraft}
                  onFlush={flushDraft}
                  onModelChange={handleModelChange}
                  modelCatalog={modelCatalogQuery.data}
                  capabilities={capabilities}
                  tokenCounts={promptTokenCounts}
                />
                <AdvancedGenerationInputs
                  draft={draft}
                  onPatch={patchDraft}
                  onFlush={flushDraft}
                  characterPresets={characterPresetsQuery.data?.items ?? EMPTY_ITEMS}
                  characterPresetsPending={characterPresetsQuery.isPending}
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
                  capabilities={capabilities}
                  tokenCounts={promptTokenCounts}
                  onOpenPositionEditor={openPositionEditor}
                  onOpenInpaintEditor={openInpaintEditor}
                />
              </PromptEditorSettingsProvider>
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
              perImageCost={estimateQuery.data?.per_image_cost ?? null}
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
              capabilities={capabilities}
              opusAllowance={resolveOpusAllowance(accountQuery.data, capabilities)}
            />
          </>
        }
        preview={
          centerWorkspace === "inpaint" && draft.i2i ? (
            <InpaintCanvasWorkspace
              key={`${draft.i2i.image.id}-${draft.i2i.inpaint?.regionToReplace.id ?? "new"}`}
              i2i={draft.i2i}
              size={draft.size}
              commitPending={canvasCommitMutation.isPending}
              onCommit={canvasCommitMutation.mutateAsync}
              onPickImageResources={() =>
                generationActions.handlePickImageResources("reference_image")
              }
              onApply={applyInpaintCanvas}
              onCancel={closePositionEditor}
            />
          ) : centerWorkspace === "positions" && capabilities && draft ? (
            <CharacterPositionWorkspace
              key={`${draft.model}-${draft.characters.map((character) => character.id).join("-")}`}
              characters={positionCharacters}
              capabilities={capabilities}
              size={draft.size}
              underlayResource={selectedSample?.resource ?? draft.i2i?.image ?? null}
              underlayStreamSrc={selectedSample?.streamSrc ?? null}
              onApply={applyCharacterPositions}
              onCancel={closePositionEditor}
            />
          ) : (
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
              onEditCharacterPositions={canEditPositions ? openPositionEditor : undefined}
              onEditSampleAsInpaint={
                selectedSample?.resource && draft.model !== "nai-diffusion-5-curated"
                  ? editSelectedOutputAsInpaint
                  : undefined
              }
            />
          )
        }
        history={
          <GenerationHistoryRail
            batches={historyBatches}
            pending={historyQuery.isPending}
            error={historyQuery.isError ? formatError(historyQuery.error) : null}
            selectedBatchId={viewBatchId}
            selectedBatchIds={selectedHistoryBatchIds}
            statusFilter={historyStatusFilter}
            offset={historyQuery.data?.offset ?? historyOffset}
            limit={historyQuery.data?.limit ?? HISTORY_PAGE_LIMIT}
            total={historyQuery.data?.total ?? 0}
            rerunPending={generationActions.rerunPending}
            deletePending={generationActions.deletePending}
            exportPending={generationActions.zipPending}
            onSelect={selectBatch}
            onToggleSelection={handleToggleHistoryBatch}
            onSelectAll={handleSelectAllHistoryBatches}
            onClearSelection={handleClearHistorySelection}
            onStatusFilterChange={(next) => {
              setSelectedHistoryBatchIds(new Set());
              setHistoryStatusFilter(next);
              setHistoryOffset(0);
            }}
            onPreviousPage={() => {
              setSelectedHistoryBatchIds(new Set());
              setHistoryOffset((value) => Math.max(0, value - HISTORY_PAGE_LIMIT));
            }}
            onNextPage={() => {
              setSelectedHistoryBatchIds(new Set());
              setHistoryOffset((value) => value + HISTORY_PAGE_LIMIT);
            }}
            onRerunSelected={generationActions.handleRerunBatch}
            onDeleteSelected={handleDeleteSelectedHistoryBatches}
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
      <GenerationHistoryDeleteConfirmation
        count={historyDeleteConfirmationOpen ? selectedHistoryBatchIds.size : 0}
        deleting={generationActions.deletePending}
        onClose={() => setHistoryDeleteConfirmationOpen(false)}
        onConfirm={handleConfirmDeleteSelectedHistoryBatches}
      />
    </div>
  );
}
