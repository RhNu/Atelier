import type { QueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppEventDto } from "@/types";

import { eventsApi } from "./client";
import { queryKeys } from "./query-keys";

export function listenToAtelierEvents(handler: (event: AppEventDto) => void): Promise<UnlistenFn> {
  return listen<AppEventDto>("atelier-event", (event) => {
    handler(event.payload);
  });
}

const EVENT_RECOVERY_PAGE_SIZE = 256;

export async function recoverAtelierEvents(
  sequence: number,
  handler: (event: AppEventDto) => void,
): Promise<number> {
  let cursor = sequence;
  for (;;) {
    const page = await eventsApi.since({ sequence: cursor, limit: EVENT_RECOVERY_PAGE_SIZE });
    for (const event of page.items) {
      if (event.sequence > cursor) {
        handler(event);
        cursor = event.sequence;
      }
    }
    if (page.items.length < EVENT_RECOVERY_PAGE_SIZE || page.next_sequence <= sequence) {
      return cursor;
    }
    sequence = page.next_sequence;
  }
}

export function applyAtelierEventInvalidations(queryClient: QueryClient, event: AppEventDto): void {
  switch (event.kind.kind) {
    case "batch_submitted":
    case "job_preparing":
    case "prompt_compiled":
    case "generation_planned":
    case "job_failed":
    case "job_succeeded":
      void queryClient.invalidateQueries({ queryKey: queryKeys.generation.root() });
      void queryClient.invalidateQueries({ queryKey: queryKeys.history.root() });
      void queryClient.invalidateQueries({ queryKey: queryKeys.account.activeSummary() });
      break;
    case "generation_stream_chunk":
      break;
    case "sample_persisted":
    case "gallery_indexed":
    case "safety_scan_failed":
    case "director_safety_scan_failed":
      void queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() });
      void queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() });
      void queryClient.invalidateQueries({ queryKey: queryKeys.history.root() });
      break;
  }

  if (event.kind.kind === "job_succeeded") {
    void queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() });
  }
}
