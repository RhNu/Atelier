import type { QueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppEventDto } from "@/types";

import { frontendLogger, reportBackgroundPromise } from "../../app/logger";
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
  frontendLogger.info("Atelier event recovery started", { sequence });
  let cursor = sequence;
  let pages = 0;
  for (;;) {
    const page = await eventsApi.since({ sequence: cursor, limit: EVENT_RECOVERY_PAGE_SIZE });
    pages += 1;
    for (const event of page.items) {
      if (event.sequence > cursor) {
        handler(event);
        cursor = event.sequence;
      }
    }
    if (page.items.length < EVENT_RECOVERY_PAGE_SIZE || page.next_sequence <= sequence) {
      frontendLogger.info("Atelier event recovery completed", {
        sequence,
        cursor,
        pages,
      });
      return cursor;
    }
    sequence = page.next_sequence;
  }
}

export function applyAtelierEventInvalidations(queryClient: QueryClient, event: AppEventDto): void {
  frontendLogger.debug("Applying Atelier event invalidations", {
    kind: event.kind.kind,
    sequence: event.sequence,
  });
  switch (event.kind.kind) {
    case "batch_submitted":
    case "job_preparing":
    case "prompt_compiled":
    case "generation_planned":
    case "job_failed":
    case "job_succeeded":
      invalidate(queryClient, queryKeys.generation.root(), event.kind.kind);
      invalidate(queryClient, queryKeys.history.root(), event.kind.kind);
      invalidate(queryClient, queryKeys.account.activeSummary(), event.kind.kind);
      break;
    case "generation_stream_chunk":
      break;
    case "sample_persisted":
    case "gallery_indexed":
    case "safety_scan_failed":
    case "director_safety_scan_failed":
      invalidate(queryClient, queryKeys.resource.root(), event.kind.kind);
      invalidate(queryClient, queryKeys.gallery.root(), event.kind.kind);
      invalidate(queryClient, queryKeys.history.root(), event.kind.kind);
      break;
  }

  if (event.kind.kind === "job_succeeded") {
    invalidate(queryClient, queryKeys.gallery.root(), event.kind.kind);
  }
}

function invalidate(
  queryClient: QueryClient,
  queryKey: readonly unknown[],
  eventKind: string,
): void {
  reportBackgroundPromise(
    queryClient.invalidateQueries({ queryKey }),
    "Atelier event cache invalidation",
    { eventKind, queryKey },
  );
}
