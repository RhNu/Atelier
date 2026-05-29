import {
  recordGenerationEvent,
  resetGenerationEventState,
  useGenerationEventStore,
} from "../features/generation/state/generation-event-store";
import type { AppEventDto } from "../types";

function event(kind: AppEventDto["kind"], sequence = 1): AppEventDto {
  return { sequence, kind };
}

describe("generation event store", () => {
  beforeEach(() => {
    resetGenerationEventState();
  });

  it("tracks stream chunks as data URLs for the active preview", () => {
    recordGenerationEvent(
      event({
        kind: "generation_stream_chunk",
        batch_id: "batch-1",
        job_id: "job-1",
        event_type: "intermediate",
        sample_index: 0,
        step_index: 4,
        generation_id: 10,
        sigma: null,
        image: "base64-frame",
      }),
    );

    expect(useGenerationEventStore.getState().activePreview).toEqual({
      kind: "stream",
      batchId: "batch-1",
      jobId: "job-1",
      sampleIndex: 0,
      stepIndex: 4,
      generationId: 10,
      eventType: "intermediate",
      src: "data:image/png;base64,base64-frame",
    });
  });

  it("promotes persisted samples to final resource previews and records terminal failures", () => {
    recordGenerationEvent(
      event({
        kind: "sample_persisted",
        batch_id: "batch-1",
        job_id: "job-1",
        sample_index: 1,
        resource: { id: "resource:generated:job-1:1", variant_id: null },
        artifact_id: "artifact-1",
      }),
    );
    recordGenerationEvent(
      event(
        {
          kind: "gallery_indexed",
          batch_id: "batch-1",
          job_id: "job-1",
          item_id: "gallery-1",
        },
        2,
      ),
    );
    recordGenerationEvent(
      event(
        {
          kind: "job_failed",
          batch_id: "batch-1",
          job_id: "job-1",
          message: "NovelAI unavailable",
        },
        3,
      ),
    );

    expect(useGenerationEventStore.getState().activePreview).toEqual({
      kind: "resource",
      batchId: "batch-1",
      jobId: "job-1",
      sampleIndex: 1,
      artifactId: "artifact-1",
      galleryItemId: "gallery-1",
      resource: { id: "resource:generated:job-1:1", variant_id: null },
    });
    expect(useGenerationEventStore.getState().lastError).toEqual({
      batchId: "batch-1",
      jobId: "job-1",
      message: "NovelAI unavailable",
    });
  });

  it("clears stale job errors when generation continues or succeeds", () => {
    recordGenerationEvent(
      event({
        kind: "job_failed",
        batch_id: "batch-1",
        job_id: "job-1",
        message: "NovelAI unavailable",
      }),
    );
    expect(useGenerationEventStore.getState().lastError?.message).toBe("NovelAI unavailable");

    recordGenerationEvent(
      event(
        {
          kind: "generation_stream_chunk",
          batch_id: "batch-1",
          job_id: "job-1",
          event_type: "intermediate",
          sample_index: 0,
          step_index: 1,
          generation_id: 11,
          sigma: null,
          image: "new-frame",
        },
        2,
      ),
    );
    expect(useGenerationEventStore.getState().lastError).toBeNull();

    recordGenerationEvent(
      event(
        {
          kind: "job_failed",
          batch_id: "batch-1",
          job_id: "job-1",
          message: "Retry failed",
        },
        3,
      ),
    );
    recordGenerationEvent(
      event(
        {
          kind: "job_succeeded",
          batch_id: "batch-1",
          job_id: "job-1",
        },
        4,
      ),
    );
    expect(useGenerationEventStore.getState().lastError).toBeNull();
  });
});
