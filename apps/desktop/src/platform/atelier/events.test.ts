import { QueryClient } from "@tanstack/react-query";

import { eventsApi } from "./client";
import { applyAtelierEventInvalidations, recoverAtelierEvents } from "./events";

describe("applyAtelierEventInvalidations", () => {
  it("invalidates generation, history, and gallery data for completed jobs", () => {
    const invalidated: unknown[] = [];
    const queryClient = new QueryClient();
    const invalidateQueries = vi
      .spyOn(queryClient, "invalidateQueries")
      .mockImplementation((options) => {
        if (options?.queryKey) {
          invalidated.push(options.queryKey);
        }
        return Promise.resolve();
      });

    applyAtelierEventInvalidations(queryClient, {
      sequence: 42,
      kind: {
        kind: "job_succeeded",
        batch_id: "batch-1",
        job_id: "job-1",
      },
    });

    expect(invalidated).toContainEqual(["workspace", "generation"]);
    expect(invalidated).toContainEqual(["workspace", "history"]);
    expect(invalidated).toContainEqual(["workspace", "gallery"]);
    expect(invalidated).toContainEqual(["app", "account", "active-summary"]);
    expect(invalidateQueries).toHaveBeenCalledTimes(4);
  });

  it("does not refetch command-backed queries for stream chunks", () => {
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

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

    expect(invalidateQueries).not.toHaveBeenCalled();
  });
});

describe("recoverAtelierEvents", () => {
  it("replays retained events after the supplied cursor", async () => {
    const items = [
      {
        sequence: 4,
        kind: { kind: "job_succeeded", batch_id: "batch-1", job_id: "job-1" } as const,
      },
      {
        sequence: 5,
        kind: { kind: "job_failed", batch_id: "batch-1", job_id: "job-2", message: "bad" } as const,
      },
    ];
    vi.spyOn(eventsApi, "since").mockResolvedValue({ items, next_sequence: 5 });
    const received: number[] = [];

    const cursor = await recoverAtelierEvents(3, (event) => received.push(event.sequence));

    expect(eventsApi.since).toHaveBeenCalledWith({ sequence: 3, limit: 256 });
    expect(received).toEqual([4, 5]);
    expect(cursor).toBe(5);
  });
});
