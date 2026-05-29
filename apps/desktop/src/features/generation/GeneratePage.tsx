import { Pause, Play, Square } from "lucide-react";
import { useCallback, useState } from "react";

import { AppIconButton, AppToolbar, EmptyState } from "../../components/ui";
import type { CompiledPromptDto, RunHistoryItemDto } from "../../types";
import { AdvancedPlaceholders } from "./components/AdvancedPlaceholders";
import { GenerationHistoryRail } from "./components/GenerationHistoryRail";
import { GenerationParamsPanel } from "./components/GenerationParamsPanel";
import { GenerationPreviewStage } from "./components/GenerationPreviewStage";
import { GenerationPromptPanel } from "./components/GenerationPromptPanel";
import {
  useCompilePromptMutation,
  useGenerationSettingsQuery,
  usePauseGenerationMutation,
  useResourceImageQuery,
  useResumeGenerationMutation,
  useStopGenerationMutation,
  useSubmitGenerationMutation,
} from "./data/useGenerationActions";
import {
  useGenerationStatusQuery,
  useLatestRunHistoryQuery,
} from "./data/useGenerationStatusQuery";
import { buildSubmitGenerationRequest, canSubmitGenerationDraft } from "./model/generation-draft";
import { useGenerationEventStore } from "./state/generation-event-store";
import { useGenerationDraft } from "./state/useGenerationDraft";

const EMPTY_HISTORY_ITEMS: ReadonlyArray<RunHistoryItemDto> = [];

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
  const historyQuery = useLatestRunHistoryQuery();
  const submitMutation = useSubmitGenerationMutation();
  const pauseMutation = usePauseGenerationMutation();
  const resumeMutation = useResumeGenerationMutation();
  const stopMutation = useStopGenerationMutation();
  const compileMutation = useCompilePromptMutation();
  const { draft, patchDraft, patchSize } = useGenerationDraft(settingsQuery.data);
  const activePreview = useGenerationEventStore((state) => state.activePreview);
  const lastError = useGenerationEventStore((state) => state.lastError);
  const selectedHistoryItemId = useGenerationEventStore((state) => state.selectedHistoryItemId);
  const selectHistoryItem = useGenerationEventStore((state) => state.selectHistoryItem);
  const finalResource = activePreview?.kind === "resource" ? activePreview.resource : null;
  const finalImageQuery = useResourceImageQuery(finalResource);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [queueError, setQueueError] = useState<string | null>(null);
  const [compileError, setCompileError] = useState<string | null>(null);
  const [positivePreview, setPositivePreview] = useState<CompiledPromptDto | null>(null);
  const [negativePreview, setNegativePreview] = useState<CompiledPromptDto | null>(null);

  const status = statusQuery.data;
  const historyItems = historyQuery.data?.items ?? EMPTY_HISTORY_ITEMS;
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
        await submitMutation.mutateAsync(buildSubmitGenerationRequest(draft));
      } catch (error) {
        setSubmitError(formatError(error));
      }
    })();
  }, [draft, submitMutation]);

  const handleCompile = useCallback(() => {
    if (!draft) {
      return;
    }

    setCompileError(null);
    setPositivePreview(null);
    setNegativePreview(null);

    void (async () => {
      try {
        if (draft.prompt.trim().length > 0) {
          setPositivePreview(
            await compileMutation.mutateAsync({ prompt: draft.prompt, max_depth: 8 }),
          );
        }
        if (draft.negativePrompt.trim().length > 0) {
          setNegativePreview(
            await compileMutation.mutateAsync({
              prompt: draft.negativePrompt,
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
      </AppToolbar>
      {queueError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {queueError}
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
            positivePreview={positivePreview}
            negativePreview={negativePreview}
            onPatch={patchDraft}
            onSubmit={handleSubmit}
            onCompile={handleCompile}
          />
          <GenerationParamsPanel draft={draft} onPatch={patchDraft} onPatchSize={patchSize} />
          <AdvancedPlaceholders />
        </aside>

        <GenerationPreviewStage
          preview={activePreview}
          finalImage={finalImageQuery.data}
          finalImagePending={finalImageQuery.isPending && Boolean(finalResource)}
          finalImageError={finalImageQuery.isError ? formatError(finalImageQuery.error) : null}
          status={status}
          statusError={statusQuery.isError ? formatError(statusQuery.error) : null}
          lastError={lastError?.message ?? null}
        />

        <GenerationHistoryRail
          items={historyItems}
          pending={historyQuery.isPending}
          error={historyQuery.isError ? formatError(historyQuery.error) : null}
          selectedItemId={selectedHistoryItemId}
          onSelect={selectHistoryItem}
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
