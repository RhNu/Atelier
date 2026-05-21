import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";

import type { AppEventDto } from "../../types";
import { queryKeys } from "./query-keys";

export function listenToAtelierEvents(handler: (event: AppEventDto) => void): Promise<UnlistenFn> {
  return listen<AppEventDto>("atelier-event", (event) => {
    handler(event.payload);
  });
}

export function applyAtelierEventInvalidations(queryClient: QueryClient, event: AppEventDto): void {
  switch (event.kind.kind) {
    case "batch_submitted":
    case "job_preparing":
    case "prompt_compiled":
    case "generation_planned":
    case "job_failed":
    case "job_succeeded":
      queryClient.invalidateQueries({ queryKey: queryKeys.generation.root() });
      queryClient.invalidateQueries({ queryKey: queryKeys.history.root() });
      break;
    case "generation_stream_chunk":
      break;
    case "sample_persisted":
    case "gallery_indexed":
    case "safety_scan_failed":
    case "director_safety_scan_failed":
      queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() });
      queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() });
      queryClient.invalidateQueries({ queryKey: queryKeys.history.root() });
      break;
  }

  if (event.kind.kind === "job_succeeded") {
    queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() });
  }
}
