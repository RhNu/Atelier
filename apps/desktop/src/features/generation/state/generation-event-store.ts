import { create } from "zustand";

import type { AppEventDto, ResourceRefDto } from "../../../types";

export type StreamGenerationPreview = {
  kind: "stream";
  batchId: string;
  jobId: string;
  sampleIndex: number;
  stepIndex: number | null;
  generationId: number;
  eventType: string;
  src: string;
};

export type ResourceGenerationPreview = {
  kind: "resource";
  batchId: string;
  jobId: string;
  sampleIndex: number;
  artifactId: string;
  galleryItemId: string | null;
  resource: ResourceRefDto;
};

export type GenerationPreview = StreamGenerationPreview | ResourceGenerationPreview;

export type GenerationRunError = {
  batchId: string;
  jobId: string;
  message: string;
};

export type GenerationEventState = {
  activeBatchId: string | null;
  activeJobId: string | null;
  activePreview: GenerationPreview | null;
  filmstrip: GenerationPreview[];
  lastError: GenerationRunError | null;
  terminalJobId: string | null;
  terminalStatus: "succeeded" | "failed" | null;
  selectedHistoryItemId: string | null;
  recordEvent: (event: AppEventDto) => void;
  selectPreview: (preview: GenerationPreview) => void;
  selectHistoryItem: (itemId: string | null) => void;
  reset: () => void;
};

const initialState = {
  activeBatchId: null,
  activeJobId: null,
  activePreview: null,
  filmstrip: [],
  lastError: null,
  terminalJobId: null,
  terminalStatus: null,
  selectedHistoryItemId: null,
} satisfies Omit<
  GenerationEventState,
  "recordEvent" | "selectPreview" | "selectHistoryItem" | "reset"
>;

export const useGenerationEventStore = create<GenerationEventState>((set) => ({
  ...initialState,
  recordEvent: (event) => {
    set((state) => applyGenerationEvent(state, event));
  },
  selectPreview: (preview) => set({ activePreview: preview }),
  selectHistoryItem: (itemId) => set({ selectedHistoryItemId: itemId }),
  reset: () => set(initialState),
}));

export function recordGenerationEvent(event: AppEventDto): void {
  useGenerationEventStore.getState().recordEvent(event);
}

export function resetGenerationEventState(): void {
  useGenerationEventStore.getState().reset();
}

function applyGenerationEvent(
  state: GenerationEventState,
  event: AppEventDto,
): Partial<GenerationEventState> {
  switch (event.kind.kind) {
    case "batch_submitted":
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: null,
        activePreview: null,
        filmstrip: [],
        lastError: null,
        terminalJobId: null,
        terminalStatus: null,
      };
    case "job_preparing":
    case "prompt_compiled":
    case "generation_planned":
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: event.kind.job_id,
        lastError: null,
      };
    case "generation_stream_chunk":
      const streamPreview: StreamGenerationPreview = {
        kind: "stream",
        batchId: event.kind.batch_id,
        jobId: event.kind.job_id,
        sampleIndex: event.kind.sample_index,
        stepIndex: event.kind.step_index,
        generationId: event.kind.generation_id,
        eventType: event.kind.event_type,
        src: `data:image/png;base64,${event.kind.image}`,
      };
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: event.kind.job_id,
        lastError: null,
        activePreview: streamPreview,
        filmstrip: appendFilmstripPreview(state.filmstrip, streamPreview),
      };
    case "sample_persisted":
      const resourcePreview: ResourceGenerationPreview = {
        kind: "resource",
        batchId: event.kind.batch_id,
        jobId: event.kind.job_id,
        sampleIndex: event.kind.sample_index,
        artifactId: event.kind.artifact_id,
        galleryItemId: null,
        resource: event.kind.resource,
      };
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: event.kind.job_id,
        lastError: null,
        activePreview: resourcePreview,
        filmstrip: appendFilmstripPreview(state.filmstrip, resourcePreview),
      };
    case "gallery_indexed":
      const galleryItemId = event.kind.item_id;
      const galleryBatchId = event.kind.batch_id;
      const galleryJobId = event.kind.job_id;
      const gallerySampleIndex = event.kind.sample_index;
      const galleryArtifactId = event.kind.artifact_id;
      const filmstrip = state.filmstrip.map((preview) =>
        preview.kind === "resource" &&
        preview.batchId === galleryBatchId &&
        preview.jobId === galleryJobId &&
        preview.sampleIndex === gallerySampleIndex &&
        preview.artifactId === galleryArtifactId
          ? { ...preview, galleryItemId }
          : preview,
      );
      const activePreview = state.activePreview;
      if (
        activePreview?.kind === "resource" &&
        activePreview.batchId === galleryBatchId &&
        activePreview.jobId === galleryJobId &&
        activePreview.sampleIndex === gallerySampleIndex &&
        activePreview.artifactId === galleryArtifactId
      ) {
        return {
          activePreview: {
            ...activePreview,
            galleryItemId: event.kind.item_id,
          },
          filmstrip,
        };
      }
      return { filmstrip };
    case "job_succeeded":
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: event.kind.job_id,
        lastError: null,
        terminalJobId: event.kind.job_id,
        terminalStatus: "succeeded",
      };
    case "job_failed":
      return {
        activeBatchId: event.kind.batch_id,
        activeJobId: event.kind.job_id,
        lastError: {
          batchId: event.kind.batch_id,
          jobId: event.kind.job_id,
          message: event.kind.message,
        },
        terminalJobId: event.kind.job_id,
        terminalStatus: "failed",
      };
    case "safety_scan_failed":
    case "director_safety_scan_failed":
      return {};
  }
}

function appendFilmstripPreview(
  current: GenerationPreview[],
  preview: GenerationPreview,
): GenerationPreview[] {
  const next = [...current.filter((item) => filmstripKey(item) !== filmstripKey(preview)), preview];
  return next.slice(-24);
}

function filmstripKey(preview: GenerationPreview): string {
  if (preview.kind === "stream") {
    return [
      preview.kind,
      preview.batchId,
      preview.jobId,
      preview.sampleIndex,
      preview.stepIndex ?? "final",
      preview.generationId,
    ].join(":");
  }
  return [
    preview.kind,
    preview.batchId,
    preview.jobId,
    preview.sampleIndex,
    preview.artifactId,
  ].join(":");
}
