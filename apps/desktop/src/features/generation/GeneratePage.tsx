/* eslint-disable max-lines-per-function */
import { useCallback, useState } from "react";

import { AppToolbar } from "../../components/ui";
import type { CompiledGenerationPromptDto, RunHistoryStatusDto } from "../../types";
import { AdvancedGenerationInputs } from "./components/AdvancedGenerationInputs";
import {
  GenerationEconomyStatus,
  GenerationLoadingState,
  GenerationSettingsError,
  QueueControls,
} from "./components/GeneratePageSupport";
import { GenerationHistoryRail } from "./components/GenerationHistoryRail";
import { GenerationParamsPanel } from "./components/GenerationParamsPanel";
import { GenerationPreviewStage } from "./components/GenerationPreviewStage";
import { GenerationPromptPanel } from "./components/GenerationPromptPanel";
import {
  useCompilePromptMutation,
  useCachedActiveSubscriptionQuery,
  useGenerationEstimateQuery,
  useGenerationSettingsQuery,
  usePauseGenerationMutation,
  usePromptPresetsQuery,
  useResourceImageQuery,
  useResumeGenerationMutation,
  useStopGenerationMutation,
  useSubmitGenerationMutation,
  useVibeDocumentsQuery,
} from "./data/useGenerationActions";
import { useGenerationStatusQuery, useRunHistoryQuery } from "./data/useGenerationStatusQuery";
import { formatGenerationError as formatError } from "./generation-page-utils";
import {
  buildSubmitGenerationBatchRequest,
  canSubmitGenerationDraft,
} from "./model/generation-draft";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";
import { useGenerationPageActions } from "./state/useGenerationPageActions";
import { useQueueActionHandlers } from "./state/useQueueActionHandlers";

const EMPTY_ITEMS: [] = [];
const HISTORY_PAGE_LIMIT = 8;

export function GeneratePage() {
  const settingsQuery = useGenerationSettingsQuery();
  const statusQuery = useGenerationStatusQuery();
  const accountQuery = useCachedActiveSubscriptionQuery();
  const submitMutation = useSubmitGenerationMutation();
  const pauseMutation = usePauseGenerationMutation();
  const resumeMutation = useResumeGenerationMutation();
  const stopMutation = useStopGenerationMutation();
  const compileMutation = useCompilePromptMutation();
  const { draft, patchDraft, patchSize } = useGenerationDraft(settingsQuery.data);
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
  const vibeDocumentsQuery = useVibeDocumentsQuery({ offset: 0, limit: 32, include_hidden: false });
  const [validationError, setValidationError] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [queueError, setQueueError] = useState<string | null>(null);
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
      return;
    }

    setValidationError(null);
    setSubmitError(null);
    void (async () => {
      try {
        await submitMutation.mutateAsync(
          buildSubmitGenerationBatchRequest(draft, undefined, { isOpus }),
        );
      } catch (error) {
        setSubmitError(formatError(error));
      }
    })();
  }, [draft, isOpus, submitMutation]);

  const handleHistoryStatusFilterChange = useCallback((status: "all" | RunHistoryStatusDto) => {
    setHistoryStatusFilter(status);
    setHistoryOffset(0);
  }, []);

  const handlePreviousHistoryPage = useCallback(() => {
    setHistoryOffset((value) => Math.max(0, value - HISTORY_PAGE_LIMIT));
  }, []);

  const handleNextHistoryPage = useCallback(() => {
    setHistoryOffset((value) => value + HISTORY_PAGE_LIMIT);
  }, []);

  const handleCompile = useCallback(() => {
    if (!draft) {
      return;
    }

    setCompileError(null);
    setCompiledPreview(null);

    void (async () => {
      try {
        if (draft.prompt.trim().length > 0 || draft.mainPresetId || draft.characters.length > 0) {
          setCompiledPreview(
            await compileMutation.mutateAsync({
              prompt: draft.prompt,
              main_preset_id: draft.mainPresetId,
              negative_prompt: draft.negativePrompt.trim().length > 0 ? draft.negativePrompt : null,
              characters: draft.characters
                .map((character) => ({
                  preset_id: character.presetId,
                  prompt: character.prompt.trim(),
                  negative_prompt:
                    character.negativePrompt.trim().length > 0 ? character.negativePrompt : null,
                  enabled: character.enabled,
                }))
                .filter(
                  (character) =>
                    character.enabled &&
                    (character.prompt.length > 0 || Boolean(character.preset_id)),
                ),
              max_depth: 8,
            }),
          );
        }
      } catch (error) {
        setCompileError(formatError(error));
      }
    })();
  }, [compileMutation, draft]);

  if (settingsQuery.isError) {
    return <GenerationSettingsError error={settingsQuery.error} />;
  }

  if (settingsQuery.isPending || !draft) {
    return <GenerationLoadingState />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Generate</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Image Workspace</h1>
        </div>
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
        <GenerationEconomyStatus
          accountPending={accountQuery.isPending}
          accountError={accountQuery.isError ? formatError(accountQuery.error) : null}
          anlasBalance={accountQuery.data?.anlas_balance ?? null}
          estimatePending={estimateQuery.isPending}
          estimateError={estimateQuery.isError ? formatError(estimateQuery.error) : null}
          estimateTotal={estimateQuery.data?.total_cost ?? null}
        />
      </AppToolbar>
      {queueError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {queueError}
        </p>
      ) : null}
      {generationActions.error ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {generationActions.error}
        </p>
      ) : null}

      <div className="grid min-h-0 flex-1 grid-cols-[420px_minmax(0,1fr)_280px] gap-3 p-3">
        <aside className="grid min-h-0 grid-rows-[minmax(0,1.1fr)_minmax(0,1fr)_auto] gap-3">
          <GenerationPromptPanel
            draft={draft}
            submitError={submitError}
            validationError={validationError}
            compileError={compileError}
            compilePending={compileMutation.isPending}
            submitPending={submitMutation.isPending}
            compiledPreview={compiledPreview}
            mainPresets={mainPresetsQuery.data?.items ?? EMPTY_ITEMS}
            mainPresetsPending={mainPresetsQuery.isPending}
            onPatch={patchDraft}
            onSubmit={handleSubmit}
            onCompile={handleCompile}
          />
          <GenerationParamsPanel draft={draft} onPatch={patchDraft} onPatchSize={patchSize} />
          <AdvancedGenerationInputs
            draft={draft}
            onPatch={patchDraft}
            characterPresets={characterPresetsQuery.data?.items ?? EMPTY_ITEMS}
            vibeDocuments={vibeDocumentsQuery.data?.items ?? EMPTY_ITEMS}
            vibePending={vibeDocumentsQuery.isPending}
            vibeError={vibeDocumentsQuery.isError ? formatError(vibeDocumentsQuery.error) : null}
            vibeImportPending={generationActions.vibeImportPending}
            vibeExportPending={generationActions.vibeExportPending}
            imageImportPending={generationActions.imageImportPending}
            vibeEnsurePending={generationActions.vibeEnsurePending}
            onPickImageResources={generationActions.handlePickImageResources}
            onPickVibeEncoding={generationActions.handlePickVibeEncoding}
            onReleaseImageResources={generationActions.handleReleaseImageResources}
            onImportVibeDocuments={generationActions.handleImportVibeDocuments}
            onExportVibeDocument={generationActions.handleExportVibeDocument}
          />
        </aside>

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
          onSelectPreview={selectPreview}
          onSavePreview={generationActions.handleSavePreview}
          onSendPreviewToDirector={generationActions.handleSendPreviewToDirector}
        />

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
          onStatusFilterChange={handleHistoryStatusFilterChange}
          onPreviousPage={handlePreviousHistoryPage}
          onNextPage={handleNextHistoryPage}
          onRerunSelected={generationActions.handleRerunSelected}
          onDeleteSelected={generationActions.handleDeleteSelected}
          onExportSelected={generationActions.handleExportSelected}
          onSendSelectedToDirector={generationActions.handleSendSelectedToDirector}
        />
      </div>
    </div>
  );
}
