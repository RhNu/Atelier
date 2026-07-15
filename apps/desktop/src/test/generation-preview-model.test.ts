import {
  buildGenerationBatchView,
  selectDefaultRequest,
} from "../features/generation/model/generation-preview-model";
import { generationPreviewKey } from "../features/generation/state/generation-event-store";
import type { GenerationHistoryBatchDetailDto, GenerationStatusDto } from "../types";

const status: GenerationStatusDto = {
  batch_id: "batch-1",
  batch_status: "running",
  current_job_id: "job-2",
  job_status: "running",
  requests: [
    { job_id: "job-1", request_index: 0, expected_samples: 4, status: "failed" },
    { job_id: "job-2", request_index: 1, expected_samples: 2, status: "running" },
  ],
};

describe("generation preview model", () => {
  it("precreates stable sample slots and overlays stream and final resources by sample index", () => {
    const view = buildGenerationBatchView({
      batchId: "batch-1",
      status,
      detail: historyDetail(),
      previews: {
        [generationPreviewKey("batch-1", "job-2", 0)]: {
          kind: "stream",
          batchId: "batch-1",
          jobId: "job-2",
          sampleIndex: 0,
          stepIndex: 3,
          generationId: 1,
          eventType: "intermediate",
          sequence: 4,
          src: "data:image/png;base64,frame",
        },
        [generationPreviewKey("batch-1", "job-2", 1)]: {
          kind: "resource",
          batchId: "batch-1",
          jobId: "job-2",
          sampleIndex: 1,
          artifactId: "artifact-2",
          galleryItemId: "gallery-2",
          resource: { id: "resource-2", variant_id: null },
          sequence: 5,
        },
      },
    });

    expect(view?.requests.map((request) => request.jobId)).toEqual(["job-1", "job-2"]);
    expect(view?.requests[0]?.samples).toHaveLength(4);
    expect(view?.requests[0]?.samples.map((sample) => sample.state)).toEqual([
      "failed",
      "failed",
      "failed",
      "failed",
    ]);
    expect(view?.requests[1]?.samples).toMatchObject([
      { sampleIndex: 0, state: "streaming", streamSrc: "data:image/png;base64,frame" },
      {
        sampleIndex: 1,
        state: "ready",
        streamSrc: null,
        resource: { id: "resource-2", variant_id: null },
      },
    ]);
  });

  it("uses pinned request first and otherwise follows the latest active request", () => {
    const view = buildGenerationBatchView({
      batchId: "batch-1",
      status,
      detail: undefined,
      previews: {},
    });
    expect(selectDefaultRequest(view, "job-1", "job-2")?.jobId).toBe("job-1");
    expect(selectDefaultRequest(view, null, "job-2")?.jobId).toBe("job-2");
  });
});

function historyDetail(): GenerationHistoryBatchDetailDto {
  return {
    batch: {
      batch_id: "batch-1",
      status: "partially_succeeded",
      title: "batch",
      last_error: null,
      created_at_ms: 1,
      updated_at_ms: 2,
      completed_at_ms: null,
      request_count: 2,
      completed_request_count: 1,
      expected_sample_count: 6,
      completed_sample_count: 0,
      outputs: [],
    },
    requests: [
      {
        run_id: "job-1",
        job_id: "job-1",
        origin_run_id: null,
        request_index: 0,
        expected_samples: 4,
        status: "failed",
        title: null,
        last_error: "failed",
        created_at_ms: 1,
        updated_at_ms: 1,
        completed_at_ms: 1,
        outputs: [],
      },
    ],
  };
}
