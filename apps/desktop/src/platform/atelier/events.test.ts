import type { QueryClient } from "@tanstack/react-query";

import { applyAtelierEventInvalidations } from "./events";

describe("applyAtelierEventInvalidations", () => {
  it("invalidates generation, history, and gallery data for completed jobs", () => {
    const invalidated: unknown[] = [];
    const queryClient = {
      invalidateQueries: vi.fn((options: { queryKey: unknown[] }) => {
        invalidated.push(options.queryKey);
      }),
    } as unknown as QueryClient;

    applyAtelierEventInvalidations(queryClient, {
      sequence: 42,
      kind: {
        kind: "job_succeeded",
        batch_id: "batch-1",
        job_id: "job-1",
      },
    });

    expect(invalidated).toContainEqual(["generation"]);
    expect(invalidated).toContainEqual(["history"]);
    expect(invalidated).toContainEqual(["gallery"]);
  });

  it("does not refetch command-backed queries for stream chunks", () => {
    const queryClient = {
      invalidateQueries: vi.fn(),
    } as unknown as QueryClient;

    applyAtelierEventInvalidations(queryClient, {
      sequence: 43,
      kind: {
        kind: "generation_stream_chunk",
        batch_id: "batch-1",
        job_id: "job-1",
        event_type: "chunk",
        sample_index: 0,
        step_index: 1,
        generation_id: 1,
        sigma: null,
        image: "data",
      },
    });

    expect(queryClient.invalidateQueries).not.toHaveBeenCalled();
  });
});
