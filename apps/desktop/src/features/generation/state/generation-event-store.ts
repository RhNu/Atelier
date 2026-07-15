import { create } from "zustand";

import type { AppEventDto, ResourceRefDto } from "@/types";

export type StreamGenerationPreview = {
  kind: "stream";
  batchId: string;
  jobId: string;
  sampleIndex: number;
  stepIndex: number | null;
  generationId: number;
  eventType: string;
  sequence: number;
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
  sequence: number;
};

export type GenerationPreview = StreamGenerationPreview | ResourceGenerationPreview;

export type GenerationRunError = {
  batchId: string;
  jobId: string;
  message: string;
};

export type GenerationFocusMode = "follow" | "pin";

export type GenerationEventState = {
  liveBatchId: string | null;
  viewBatchId: string | null;
  latestJobId: string | null;
  selectedJobId: string | null;
  focusedSampleIndex: number | null;
  focusMode: GenerationFocusMode;
  previews: Record<string, GenerationPreview>;
  lastError: GenerationRunError | null;
  terminalJobId: string | null;
  terminalStatus: "succeeded" | "failed" | null;
  recordEvent: (event: AppEventDto) => void;
  syncActiveBatch: (batchId: string | null, currentJobId: string | null) => void;
  selectBatch: (batchId: string) => void;
  selectRequest: (jobId: string) => void;
  focusSample: (jobId: string, sampleIndex: number) => void;
  showRequestGrid: () => void;
  resumeFollow: () => void;
  reset: () => void;
};

const initialState = {
  liveBatchId: null,
  viewBatchId: null,
  latestJobId: null,
  selectedJobId: null,
  focusedSampleIndex: null,
  focusMode: "follow",
  previews: {},
  lastError: null,
  terminalJobId: null,
  terminalStatus: null,
} satisfies Omit<
  GenerationEventState,
  | "recordEvent"
  | "syncActiveBatch"
  | "selectBatch"
  | "selectRequest"
  | "focusSample"
  | "showRequestGrid"
  | "resumeFollow"
  | "reset"
>;

export const useGenerationEventStore = create<GenerationEventState>((set) => ({
  ...initialState,
  recordEvent: (event) => set((state) => applyGenerationEvent(state, event)),
  syncActiveBatch: (batchId, currentJobId) =>
    set((state) => {
      if (!batchId) {
        return { liveBatchId: null };
      }
      if (state.focusMode !== "follow") {
        return { liveBatchId: batchId, latestJobId: currentJobId ?? state.latestJobId };
      }
      return {
        liveBatchId: batchId,
        viewBatchId: batchId,
        latestJobId: currentJobId ?? state.latestJobId,
        selectedJobId: currentJobId ?? state.selectedJobId,
      };
    }),
  selectBatch: (batchId) =>
    set({
      viewBatchId: batchId,
      selectedJobId: null,
      focusedSampleIndex: null,
      focusMode: "pin",
    }),
  selectRequest: (jobId) =>
    set({ selectedJobId: jobId, focusedSampleIndex: null, focusMode: "pin" }),
  focusSample: (jobId, sampleIndex) =>
    set({ selectedJobId: jobId, focusedSampleIndex: sampleIndex, focusMode: "pin" }),
  showRequestGrid: () => set({ focusedSampleIndex: null }),
  resumeFollow: () =>
    set((state) => ({
      viewBatchId: state.liveBatchId,
      selectedJobId: state.latestJobId,
      focusedSampleIndex: null,
      focusMode: "follow",
    })),
  reset: () => set(initialState),
}));

export function recordGenerationEvent(event: AppEventDto): void {
  useGenerationEventStore.getState().recordEvent(event);
}

export function resetGenerationEventState(): void {
  useGenerationEventStore.getState().reset();
}

export function generationPreviewKey(batchId: string, jobId: string, sampleIndex: number): string {
  return `${batchId}:${jobId}:${sampleIndex}`;
}

function applyGenerationEvent(
  state: GenerationEventState,
  event: AppEventDto,
): Partial<GenerationEventState> {
  switch (event.kind.kind) {
    case "batch_submitted": {
      const followingLive = state.focusMode === "follow";
      return {
        liveBatchId: event.kind.batch_id,
        viewBatchId: followingLive ? event.kind.batch_id : state.viewBatchId,
        latestJobId: null,
        selectedJobId: followingLive ? null : state.selectedJobId,
        focusedSampleIndex: followingLive ? null : state.focusedSampleIndex,
        previews: {},
        lastError: null,
        terminalJobId: null,
        terminalStatus: null,
      };
    }
    case "job_preparing":
    case "prompt_compiled":
    case "generation_planned":
      return followJob(state, event.kind.batch_id, event.kind.job_id, {
        lastError: null,
      });
    case "generation_stream_chunk": {
      const preview: StreamGenerationPreview = {
        kind: "stream",
        batchId: event.kind.batch_id,
        jobId: event.kind.job_id,
        sampleIndex: event.kind.sample_index,
        stepIndex: event.kind.step_index,
        generationId: event.kind.generation_id,
        eventType: event.kind.event_type,
        sequence: event.sequence,
        src: `data:image/png;base64,${event.kind.image}`,
      };
      return followJob(state, preview.batchId, preview.jobId, {
        previews: upsertPreview(state.previews, preview),
        lastError: null,
      });
    }
    case "sample_persisted": {
      const preview: ResourceGenerationPreview = {
        kind: "resource",
        batchId: event.kind.batch_id,
        jobId: event.kind.job_id,
        sampleIndex: event.kind.sample_index,
        artifactId: event.kind.artifact_id,
        galleryItemId: null,
        resource: event.kind.resource,
        sequence: event.sequence,
      };
      return followJob(state, preview.batchId, preview.jobId, {
        previews: upsertPreview(state.previews, preview),
        lastError: null,
      });
    }
    case "gallery_indexed": {
      const key = generationPreviewKey(
        event.kind.batch_id,
        event.kind.job_id,
        event.kind.sample_index,
      );
      const preview = state.previews[key];
      if (preview?.kind !== "resource" || preview.artifactId !== event.kind.artifact_id) {
        return {};
      }
      return {
        previews: {
          ...state.previews,
          [key]: { ...preview, galleryItemId: event.kind.item_id, sequence: event.sequence },
        },
      };
    }
    case "job_succeeded":
      return {
        latestJobId: event.kind.job_id,
        lastError: null,
        terminalJobId: event.kind.job_id,
        terminalStatus: "succeeded",
      };
    case "job_failed":
      return {
        latestJobId: event.kind.job_id,
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

function followJob(
  state: GenerationEventState,
  batchId: string,
  jobId: string,
  update: Partial<GenerationEventState>,
): Partial<GenerationEventState> {
  const followingLive = state.focusMode === "follow";
  return {
    ...update,
    liveBatchId: batchId,
    latestJobId: jobId,
    viewBatchId: followingLive ? batchId : state.viewBatchId,
    selectedJobId: followingLive ? jobId : state.selectedJobId,
    focusedSampleIndex: followingLive ? null : state.focusedSampleIndex,
  };
}

function upsertPreview(
  previews: Record<string, GenerationPreview>,
  preview: GenerationPreview,
): Record<string, GenerationPreview> {
  const key = generationPreviewKey(preview.batchId, preview.jobId, preview.sampleIndex);
  const next = { ...previews, [key]: preview };
  const keys = Object.keys(next);
  if (keys.length <= 32) {
    return next;
  }
  keys
    .sort((left, right) => (next[left]?.sequence ?? 0) - (next[right]?.sequence ?? 0))
    .slice(0, keys.length - 32)
    .forEach((oldest) => delete next[oldest]);
  return next;
}
