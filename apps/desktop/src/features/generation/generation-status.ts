import type { TFunction } from "i18next";

const STATUS_TRANSLATION_KEYS = {
  queued: "status.queued",
  preparing: "status.preparing",
  running: "status.running",
  waiting: "status.waiting",
  paused: "status.paused",
  succeeded: "status.succeeded",
  partially_succeeded: "status.partially_succeeded",
  partial: "status.partial",
  failed: "status.failed",
  skipped: "status.skipped",
  stopped: "status.stopped",
  pending: "status.pending",
  streaming: "status.streaming",
  ready: "status.ready",
  missing: "status.missing",
  complete: "status.complete",
} as const;

export function translateGenerationStatus(translate: TFunction<"generation">, status: string) {
  return isGenerationStatus(status) ? translate(STATUS_TRANSLATION_KEYS[status]) : status;
}

export function shortGenerationStatus(status: string): string {
  return status === "partially_succeeded" ? "partial" : status;
}

function isGenerationStatus(status: string): status is keyof typeof STATUS_TRANSLATION_KEYS {
  return Object.hasOwn(STATUS_TRANSLATION_KEYS, status);
}
