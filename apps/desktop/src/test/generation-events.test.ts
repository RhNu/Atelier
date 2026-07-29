import {
  generationPreviewKey,
  recordGenerationEvent,
  resetGenerationEventState,
  useGenerationEventStore,
} from "../features/generation/state/generation-event-store";
import type { AppEventDto } from "../types";

function event(kind: AppEventDto["kind"], sequence = 1): AppEventDto {
  return { sequence, kind };
}

function streamChunk(
  batchId: string,
  jobId: string,
  sampleIndex: number,
  image: string,
  sequence: number,
): AppEventDto {
  return event(
    {
      kind: "generation_stream_chunk",
      batch_id: batchId,
      job_id: jobId,
      event_type: "intermediate",
      sample_index: sampleIndex,
      step_index: sequence,
      generation_id: sequence,
      sigma: null,
      image,
    },
    sequence,
  );
}

describe("generation event store", () => {
  beforeEach(() => resetGenerationEventState());

  it("replaces consecutive chunks for one sample without adding preview entries", () => {
    recordGenerationEvent(streamChunk("batch-1", "job-1", 0, "frame-1", 1));
    recordGenerationEvent(streamChunk("batch-1", "job-1", 0, "frame-2", 2));

    const state = useGenerationEventStore.getState();
    expect(Object.keys(state.previews)).toEqual([generationPreviewKey("batch-1", "job-1", 0)]);
    expect(state.previews[generationPreviewKey("batch-1", "job-1", 0)]).toMatchObject({
      kind: "stream",
      src: "data:image/png;base64,frame-2",
      sequence: 2,
    });
  });

  it("updates different request and sample slots independently", () => {
    recordGenerationEvent(streamChunk("batch-1", "job-1", 0, "a", 1));
    recordGenerationEvent(streamChunk("batch-1", "job-1", 1, "b", 2));
    recordGenerationEvent(streamChunk("batch-1", "job-2", 0, "c", 3));

    const previews = useGenerationEventStore.getState().previews;
    expect(Object.keys(previews)).toHaveLength(3);
    expect(previews[generationPreviewKey("batch-1", "job-1", 1)]).toMatchObject({
      src: "data:image/png;base64,b",
    });
    expect(previews[generationPreviewKey("batch-1", "job-2", 0)]).toMatchObject({
      src: "data:image/png;base64,c",
    });
  });

  it("replaces streaming base64 with the final resource and attaches its gallery item", () => {
    recordGenerationEvent(streamChunk("batch-1", "job-1", 1, "large-frame", 1));
    recordGenerationEvent(
      event(
        {
          kind: "sample_persisted",
          batch_id: "batch-1",
          job_id: "job-1",
          sample_index: 1,
          resource: { id: "resource:generated:job-1:1", variant_id: null },
          artifact_id: "artifact-1",
        },
        2,
      ),
    );
    recordGenerationEvent(
      event(
        {
          kind: "gallery_indexed",
          batch_id: "batch-1",
          job_id: "job-1",
          sample_index: 1,
          artifact_id: "artifact-1",
          item_id: "gallery-1",
        },
        3,
      ),
    );

    const preview =
      useGenerationEventStore.getState().previews[generationPreviewKey("batch-1", "job-1", 1)];
    expect(preview).toEqual({
      kind: "resource",
      batchId: "batch-1",
      jobId: "job-1",
      sampleIndex: 1,
      artifactId: "artifact-1",
      galleryItemId: "gallery-1",
      resource: { id: "resource:generated:job-1:1", variant_id: null },
      sequence: 3,
    });
    expect(JSON.stringify(preview)).not.toContain("large-frame");
  });

  it("follows the latest request until the user pins a request, then resumes explicitly", () => {
    recordGenerationEvent(streamChunk("batch-1", "job-1", 0, "a", 1));
    expect(useGenerationEventStore.getState()).toMatchObject({
      viewBatchId: "batch-1",
      selectedJobId: "job-1",
      focusMode: "follow",
    });

    useGenerationEventStore.getState().selectRequest("job-1");
    recordGenerationEvent(streamChunk("batch-1", "job-2", 0, "b", 2));
    expect(useGenerationEventStore.getState()).toMatchObject({
      selectedJobId: "job-1",
      latestJobId: "job-2",
      focusMode: "pin",
    });

    useGenerationEventStore.getState().resumeFollow();
    expect(useGenerationEventStore.getState()).toMatchObject({
      viewBatchId: "batch-1",
      selectedJobId: "job-2",
      focusMode: "follow",
    });
  });

  it("keeps a selected history batch pinned while live events continue", () => {
    useGenerationEventStore.getState().selectBatch("history-batch");
    recordGenerationEvent(streamChunk("live-batch", "live-job", 0, "frame", 1));

    expect(useGenerationEventStore.getState()).toMatchObject({
      liveBatchId: "live-batch",
      viewBatchId: "history-batch",
      latestJobId: "live-job",
      selectedJobId: null,
      focusMode: "pin",
    });
    expect(
      useGenerationEventStore.getState().previews[
        generationPreviewKey("live-batch", "live-job", 0)
      ],
    ).toBeDefined();
  });

  it("follows a newly submitted batch even when an older batch was pinned", () => {
    useGenerationEventStore.getState().selectBatch("history-batch");
    recordGenerationEvent(
      event({
        kind: "batch_submitted",
        batch_id: "new-batch",
      }),
    );

    expect(useGenerationEventStore.getState()).toMatchObject({
      liveBatchId: "new-batch",
      viewBatchId: "new-batch",
      selectedJobId: null,
      focusedSampleIndex: null,
      focusMode: "follow",
    });

    recordGenerationEvent(streamChunk("new-batch", "new-job", 0, "frame", 2));
    expect(useGenerationEventStore.getState()).toMatchObject({
      viewBatchId: "new-batch",
      selectedJobId: "new-job",
      focusMode: "follow",
    });
  });

  it("keeps the preview map bounded to the configured 8 by 4 slots", () => {
    for (let index = 0; index < 40; index += 1) {
      recordGenerationEvent(streamChunk("batch-1", `job-${index}`, 0, String(index), index + 1));
    }
    const previews = useGenerationEventStore.getState().previews;
    expect(Object.keys(previews)).toHaveLength(32);
    expect(previews[generationPreviewKey("batch-1", "job-0", 0)]).toBeUndefined();
    expect(previews[generationPreviewKey("batch-1", "job-39", 0)]).toBeDefined();
  });
});
