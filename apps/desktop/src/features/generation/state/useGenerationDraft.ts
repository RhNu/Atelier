import { useCallback, useEffect, useRef, useState } from "react";

import { describeError, frontendLogger, reportBackgroundPromise } from "@/app/logger";
import type { VersionedGenerationDraftDto, WorkspaceSettingsDto } from "@/types";

import {
  createGenerationDraft,
  generationDraftFromDto,
  type GenerationDraft,
} from "../model/generation-draft";

export type GenerationDraftPersistMode = "debounced" | "immediate";
export type GenerationDraftPatchOptions = {
  persist?: GenerationDraftPersistMode;
};

type UseGenerationDraftOptions = {
  settings: WorkspaceSettingsDto | undefined;
  storedDraft: VersionedGenerationDraftDto | null | undefined;
  sourceReady: boolean;
  saveDraft: (
    draft: GenerationDraft,
    expectedRevision: number,
  ) => Promise<VersionedGenerationDraftDto>;
};

const SAVE_DEBOUNCE_MS = 250;

// Draft hydration and its serialized, debounced CAS save queue form one state machine.
// eslint-disable-next-line max-lines-per-function
export function useGenerationDraft({
  settings,
  storedDraft,
  sourceReady,
  saveDraft,
}: UseGenerationDraftOptions) {
  const [draft, setDraft] = useState<GenerationDraft | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const latestDraftRef = useRef<GenerationDraft | null>(null);
  const revisionRef = useRef(0);
  const pendingDraftRef = useRef<GenerationDraft | null>(null);
  const failedDraftRef = useRef<GenerationDraft | null>(null);
  const saveDraftRef = useRef(saveDraft);
  const saveTimerRef = useRef<number | null>(null);
  const saveInFlightRef = useRef(false);
  const mountedRef = useRef(true);
  const hydratedSettingsRef = useRef<WorkspaceSettingsDto | null>(null);

  saveDraftRef.current = saveDraft;

  const drainSaveQueue = useCallback(async () => {
    if (saveInFlightRef.current) {
      return;
    }
    saveInFlightRef.current = true;
    frontendLogger.debug("Generation draft save queue started");
    try {
      while (pendingDraftRef.current) {
        const next = pendingDraftRef.current;
        pendingDraftRef.current = null;
        frontendLogger.debug("Generation draft save started");
        try {
          const saved = await saveDraftRef.current(next, revisionRef.current);
          revisionRef.current = saved.revision;
          failedDraftRef.current = null;
          frontendLogger.info("Generation draft saved");
          if (mountedRef.current) {
            setSaveError(null);
          }
        } catch (error) {
          failedDraftRef.current = next;
          logGenerationDraftSaveFailure(error);
          if (mountedRef.current) {
            setSaveError(formatSaveError(error));
          }
          break;
        }
      }
    } finally {
      saveInFlightRef.current = false;
      frontendLogger.debug("Generation draft save queue finished");
    }
  }, []);

  const queueSave = useCallback(
    (next: GenerationDraft, mode: GenerationDraftPersistMode = "debounced") => {
      pendingDraftRef.current = next;
      if (saveTimerRef.current !== null) {
        window.clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
      }
      if (mode === "immediate") {
        reportBackgroundPromise(drainSaveQueue(), "Drain generation draft save queue");
        return;
      }
      saveTimerRef.current = window.setTimeout(() => {
        saveTimerRef.current = null;
        reportBackgroundPromise(drainSaveQueue(), "Drain generation draft save queue");
      }, SAVE_DEBOUNCE_MS);
    },
    [drainSaveQueue],
  );

  useEffect(() => {
    if (!sourceReady || !settings || hydratedSettingsRef.current === settings) {
      return;
    }
    if (saveTimerRef.current !== null) {
      window.clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }
    pendingDraftRef.current = null;
    failedDraftRef.current = null;
    setSaveError(null);
    const next = storedDraft
      ? generationDraftFromDto(storedDraft.draft)
      : createGenerationDraft(settings);
    revisionRef.current = storedDraft?.revision ?? 0;
    hydratedSettingsRef.current = settings;
    latestDraftRef.current = next;
    setDraft(next);
  }, [settings, sourceReady, storedDraft]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (saveTimerRef.current !== null) {
        window.clearTimeout(saveTimerRef.current);
      }
      const pending = pendingDraftRef.current;
      if (pending) {
        reportPendingGenerationDraftSave(saveDraftRef.current(pending, revisionRef.current));
      }
    };
  }, []);

  const replaceDraft = useCallback(
    (next: GenerationDraft, options?: GenerationDraftPatchOptions) => {
      latestDraftRef.current = next;
      setDraft(next);
      queueSave(next, options?.persist);
    },
    [queueSave],
  );

  const patchDraft = useCallback(
    (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => {
      const current = latestDraftRef.current;
      if (!current) {
        return;
      }
      replaceDraft({ ...current, ...patch }, options);
    },
    [replaceDraft],
  );

  const patchSize = useCallback(
    (patch: Partial<GenerationDraft["size"]>, options?: GenerationDraftPatchOptions) => {
      const current = latestDraftRef.current;
      if (!current) {
        return;
      }
      replaceDraft({ ...current, size: { ...current.size, ...patch } }, options);
    },
    [replaceDraft],
  );

  const flushDraft = useCallback(() => {
    const current = latestDraftRef.current;
    if (current) {
      queueSave(current, "immediate");
    }
  }, [queueSave]);

  const retrySave = useCallback(() => {
    const current = failedDraftRef.current ?? latestDraftRef.current;
    if (current) {
      queueSave(current, "immediate");
    }
  }, [queueSave]);

  const resetDraft = useCallback(
    (next: GenerationDraft) => {
      revisionRef.current = 0;
      replaceDraft(next, { persist: "immediate" });
    },
    [replaceDraft],
  );

  return {
    draft,
    patchDraft,
    patchSize,
    replaceDraft,
    resetDraft,
    flushDraft,
    retrySave,
    saveError,
  };
}

function formatSaveError(error: unknown): string {
  return error instanceof Error ? error.message : "Generation draft could not be saved.";
}

function logGenerationDraftSaveFailure(error: unknown): void {
  frontendLogger.error("Generation draft save failed", { error: describeError(error) });
}

function reportPendingGenerationDraftSave(promise: Promise<unknown>): void {
  reportBackgroundPromise(promise, "Persist pending generation draft on unmount");
}
