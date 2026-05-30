/* eslint-disable max-lines, max-lines-per-function */
import { Pause, Play, Square } from "lucide-react";
import { useCallback, useState } from "react";

import { AppIconButton, AppToolbar, EmptyState } from "../../components/ui";
import type {
  CompiledGenerationPromptDto,
  PromptPresetDto,
  RunHistoryItemDto,
  RunHistoryOutputDto,
  RunHistoryStatusDto,
  VibeDocumentEntryDto,
} from "../../types";
import { setDirectorHandoffInput } from "../director/state/director-handoff-store";
import { navigateToDirector } from "../director/state/navigate-to-director";
import { AdvancedGenerationInputs } from "./components/AdvancedGenerationInputs";
import { GenerationHistoryRail } from "./components/GenerationHistoryRail";
import { GenerationParamsPanel } from "./components/GenerationParamsPanel";
import { GenerationPreviewStage } from "./components/GenerationPreviewStage";
import { GenerationPromptPanel } from "./components/GenerationPromptPanel";
import {
  useCompilePromptMutation,
  useActiveAccountProbeQuery,
  useDeleteRunHistoryMutation,
  useGalleryImageReferenceMutation,
  useGenerationEstimateQuery,
  useGenerationSettingsQuery,
  useEnsureVibeEncodingFromResourceMutation,
  useImportVibeDocumentsMutation,
  usePauseGenerationMutation,
  usePickImageResourcesMutation,
  usePromptPresetsQuery,
  useRerunGenerationMutation,
  useResourceImageQuery,
  useResumeGenerationMutation,
  useSaveResourceImageMutation,
  useStopGenerationMutation,
  useSubmitGenerationMutation,
  useExportVibeDocumentMutation,
  useVibeDocumentsQuery,
} from "./data/useGenerationActions";
import { useGenerationStatusQuery, useRunHistoryQuery } from "./data/useGenerationStatusQuery";
import {
  buildSubmitGenerationBatchRequest,
  canSubmitGenerationDraft,
  createGenerationRunIds,
} from "./model/generation-draft";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";

const EMPTY_HISTORY_ITEMS: ReadonlyArray<RunHistoryItemDto> = [];
const EMPTY_VIBE_DOCUMENTS: ReadonlyArray<VibeDocumentEntryDto> = [];
const EMPTY_PROMPT_PRESETS: ReadonlyArray<PromptPresetDto> = [];
const HISTORY_PAGE_LIMIT = 8;

type QueueControlsProps = {
  canPause: boolean;
  canResume: boolean;
  canStop: boolean;
  pausePending: boolean;
  resumePending: boolean;
  stopPending: boolean;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
};

type QueueActionHandlersProps = {
  pause: () => Promise<unknown>;
  resume: () => Promise<unknown>;
  stop: () => Promise<unknown>;
  setQueueError: (error: string | null) => void;
};

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function GeneratePage() {
  const settingsQuery = useGenerationSettingsQuery();
  const statusQuery = useGenerationStatusQuery();
  const accountQuery = useActiveAccountProbeQuery();
  const submitMutation = useSubmitGenerationMutation();
  const rerunMutation = useRerunGenerationMutation();
  const deleteHistoryMutation = useDeleteRunHistoryMutation();
  const saveResourceMutation = useSaveResourceImageMutation();
  const imageReferenceMutation = useGalleryImageReferenceMutation();
  const imageImportMutation = usePickImageResourcesMutation();
  const ensureVibeEncodingMutation = useEnsureVibeEncodingFromResourceMutation();
  const importVibeMutation = useImportVibeDocumentsMutation();
  const exportVibeMutation = useExportVibeDocumentMutation();
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
  const [historyActionError, setHistoryActionError] = useState<string | null>(null);
  const [compiledPreview, setCompiledPreview] = useState<CompiledGenerationPromptDto | null>(null);

  const status = statusQuery.data;
  const historyItems = historyQuery.data?.items ?? EMPTY_HISTORY_ITEMS;
  const selectedHistoryItem =
    historyItems.find((item) => item.run_id === selectedHistoryItemId) ?? null;
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

  const handleRerunSelected = useCallback(() => {
    if (!selectedHistoryItem || rerunMutation.isPending) {
      return;
    }
    const ids = createGenerationRunIds(1);
    const jobId = ids.jobIds[0];
    if (!jobId) {
      return;
    }
    setHistoryActionError(null);
    void rerunMutation
      .mutateAsync({
        run_id: selectedHistoryItem.run_id,
        batch_id: ids.batchId,
        job_id: jobId,
      })
      .catch((error: unknown) => setHistoryActionError(formatError(error)));
  }, [rerunMutation, selectedHistoryItem]);

  const handleDeleteSelected = useCallback(() => {
    if (!selectedHistoryItem || deleteHistoryMutation.isPending) {
      return;
    }
    setHistoryActionError(null);
    void deleteHistoryMutation
      .mutateAsync([selectedHistoryItem.run_id])
      .then(() => selectHistoryItem(null))
      .catch((error: unknown) => setHistoryActionError(formatError(error)));
  }, [deleteHistoryMutation, selectHistoryItem, selectedHistoryItem]);

  const handleExportOutput = useCallback(
    (output: RunHistoryOutputDto | null, suggestedName: string) => {
      if (!output || saveResourceMutation.isPending) {
        return;
      }
      setHistoryActionError(null);
      void saveResourceMutation
        .mutateAsync({
          resource: output.resource,
          suggested_file_name: suggestedName,
        })
        .catch((error: unknown) => setHistoryActionError(formatError(error)));
    },
    [saveResourceMutation],
  );

  const handleExportSelected = useCallback(() => {
    handleExportOutput(
      preferredHistoryOutput(selectedHistoryItem),
      selectedHistoryItem ? `${selectedHistoryItem.run_id}-sample` : "generation",
    );
  }, [handleExportOutput, selectedHistoryItem]);

  const handleSendOutputToDirector = useCallback(
    (output: RunHistoryOutputDto | null) => {
      if (!output?.item_id || imageReferenceMutation.isPending) {
        return;
      }
      setHistoryActionError(null);
      void imageReferenceMutation
        .mutateAsync({ item_id: output.item_id, target: "director" })
        .then((reference) => {
          setDirectorHandoffInput(reference.resource);
          navigateToDirector();
        })
        .catch((error: unknown) => setHistoryActionError(formatError(error)));
    },
    [imageReferenceMutation],
  );

  const handleSendSelectedToDirector = useCallback(() => {
    handleSendOutputToDirector(preferredHistoryOutput(selectedHistoryItem));
  }, [handleSendOutputToDirector, selectedHistoryItem]);

  const handleSavePreview = useCallback(() => {
    if (activePreview?.kind !== "resource") {
      return;
    }
    setHistoryActionError(null);
    void saveResourceMutation
      .mutateAsync({
        resource: activePreview.resource,
        suggested_file_name: `${activePreview.jobId}-sample-${activePreview.sampleIndex}`,
      })
      .catch((error: unknown) => setHistoryActionError(formatError(error)));
  }, [activePreview, saveResourceMutation]);

  const handleSendPreviewToDirector = useCallback(() => {
    if (activePreview?.kind !== "resource" || !activePreview.galleryItemId) {
      return;
    }
    setHistoryActionError(null);
    void imageReferenceMutation
      .mutateAsync({ item_id: activePreview.galleryItemId, target: "director" })
      .then((reference) => {
        setDirectorHandoffInput(reference.resource);
        navigateToDirector();
      })
      .catch((error: unknown) => setHistoryActionError(formatError(error)));
  }, [activePreview, imageReferenceMutation]);

  const handleImportVibeDocuments = useCallback(() => {
    setHistoryActionError(null);
    void importVibeMutation
      .mutateAsync()
      .catch((error: unknown) => setHistoryActionError(formatError(error)));
  }, [importVibeMutation]);

  const handlePickImageResources = useCallback(
    async (kind: "source_image" | "reference_image") => {
      const imported = await imageImportMutation.mutateAsync({ kind, extensions: [] });
      return imported.map((item) => item.resource);
    },
    [imageImportMutation],
  );

  const handlePickVibeEncoding = useCallback(async () => {
    if (!draft) {
      return null;
    }
    const [imported] = await imageImportMutation.mutateAsync({
      kind: "control_net_image",
      extensions: [],
    });
    if (!imported) {
      return null;
    }
    return ensureVibeEncodingMutation.mutateAsync({
      resource: imported.resource,
      model: draft.model,
      informationExtracted: 1,
    });
  }, [draft, ensureVibeEncodingMutation, imageImportMutation]);

  const handleExportVibeDocument = useCallback(
    (vibeId: string) => {
      setHistoryActionError(null);
      void exportVibeMutation
        .mutateAsync([vibeId])
        .catch((error: unknown) => setHistoryActionError(formatError(error)));
    },
    [exportVibeMutation],
  );

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
      {historyActionError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {historyActionError}
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
            mainPresets={mainPresetsQuery.data?.items ?? EMPTY_PROMPT_PRESETS}
            mainPresetsPending={mainPresetsQuery.isPending}
            onPatch={patchDraft}
            onSubmit={handleSubmit}
            onCompile={handleCompile}
          />
          <GenerationParamsPanel draft={draft} onPatch={patchDraft} onPatchSize={patchSize} />
          <AdvancedGenerationInputs
            draft={draft}
            onPatch={patchDraft}
            characterPresets={characterPresetsQuery.data?.items ?? EMPTY_PROMPT_PRESETS}
            vibeDocuments={vibeDocumentsQuery.data?.items ?? EMPTY_VIBE_DOCUMENTS}
            vibePending={vibeDocumentsQuery.isPending}
            vibeError={vibeDocumentsQuery.isError ? formatError(vibeDocumentsQuery.error) : null}
            vibeImportPending={importVibeMutation.isPending}
            vibeExportPending={exportVibeMutation.isPending}
            imageImportPending={imageImportMutation.isPending}
            vibeEnsurePending={ensureVibeEncodingMutation.isPending}
            onPickImageResources={handlePickImageResources}
            onPickVibeEncoding={handlePickVibeEncoding}
            onImportVibeDocuments={handleImportVibeDocuments}
            onExportVibeDocument={handleExportVibeDocument}
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
          savePending={saveResourceMutation.isPending}
          handoffPending={imageReferenceMutation.isPending}
          onSelectPreview={selectPreview}
          onSavePreview={handleSavePreview}
          onSendPreviewToDirector={handleSendPreviewToDirector}
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
          rerunPending={rerunMutation.isPending}
          deletePending={deleteHistoryMutation.isPending}
          exportPending={saveResourceMutation.isPending}
          handoffPending={imageReferenceMutation.isPending}
          onSelect={selectHistoryItem}
          onStatusFilterChange={handleHistoryStatusFilterChange}
          onPreviousPage={handlePreviousHistoryPage}
          onNextPage={handleNextHistoryPage}
          onRerunSelected={handleRerunSelected}
          onDeleteSelected={handleDeleteSelected}
          onExportSelected={handleExportSelected}
          onSendSelectedToDirector={handleSendSelectedToDirector}
        />
      </div>
    </div>
  );
}

function useQueueActionHandlers({ pause, resume, stop, setQueueError }: QueueActionHandlersProps) {
  const runQueueCommand = useCallback(
    (command: () => Promise<unknown>) => {
      setQueueError(null);
      void command().catch((error: unknown) => {
        setQueueError(formatError(error));
      });
    },
    [setQueueError],
  );
  const handlePause = useCallback(() => {
    runQueueCommand(pause);
  }, [pause, runQueueCommand]);
  const handleResume = useCallback(() => {
    runQueueCommand(resume);
  }, [resume, runQueueCommand]);
  const handleStop = useCallback(() => {
    runQueueCommand(stop);
  }, [runQueueCommand, stop]);

  return { handlePause, handleResume, handleStop };
}

function preferredHistoryOutput(item: RunHistoryItemDto | null): RunHistoryOutputDto | null {
  if (!item) {
    return null;
  }
  return (
    item.outputs.find((output) => output.asset_role === "original") ??
    item.outputs.find((output) => output.asset_role === "primary") ??
    item.outputs[0] ??
    null
  );
}

function GenerationEconomyStatus({
  accountPending,
  accountError,
  anlasBalance,
  estimatePending,
  estimateError,
  estimateTotal,
}: {
  accountPending: boolean;
  accountError: string | null;
  anlasBalance: number | null;
  estimatePending: boolean;
  estimateError: string | null;
  estimateTotal: number | null;
}) {
  const accountLabel = accountError
    ? accountError
    : accountPending
      ? "Account"
      : `${anlasBalance ?? 0} Anlas`;
  const estimateLabel = estimateError
    ? "Estimate unavailable"
    : estimatePending
      ? "Estimating"
      : `${estimateTotal ?? 0} planned`;

  return (
    <div className="flex items-center gap-2 text-xs text-app-muted">
      <span className="border border-app-border bg-app-panel px-2 py-1">{accountLabel}</span>
      <span className="border border-app-border bg-app-panel px-2 py-1">{estimateLabel}</span>
    </div>
  );
}

function GenerationLoadingState() {
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState title="Loading generation defaults" />
    </div>
  );
}

function GenerationSettingsError({ error }: { error: unknown }) {
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState title="Generation settings unavailable" description={formatError(error)} />
    </div>
  );
}

function QueueControls({
  canPause,
  canResume,
  canStop,
  pausePending,
  resumePending,
  stopPending,
  onPause,
  onResume,
  onStop,
}: QueueControlsProps) {
  return (
    <div className="flex items-center gap-2">
      <AppIconButton
        icon={Pause}
        label="Pause queue"
        disabled={!canPause || pausePending}
        onClick={onPause}
      />
      <AppIconButton
        icon={Play}
        label="Resume queue"
        disabled={!canResume || resumePending}
        onClick={onResume}
      />
      <AppIconButton
        icon={Square}
        label="Stop queue"
        disabled={!canStop || stopPending}
        onClick={onStop}
      />
    </div>
  );
}
