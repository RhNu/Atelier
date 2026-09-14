import { useCallback, useEffect, useRef, useState } from "react";

import { describeError, frontendLogger, reportBackgroundPromise } from "@/app/logger";
import type { VersionedGenerationDraftDto, WorkspaceSettingsDto } from "@/types";

import {
  createGenerationDraft,
  generationDraftFromDto,
  type GenerationDraft,
} from "../model/generation-draft";
import { registerGenerationDraftFlush, trackGenerationDraftSave } from "./draft-save-barrier";

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
  const saveInFlightRef = useRef<Promise<void> | null>(null);
  const mountedRef = useRef(true);
  const hydratedSettingsRef = useRef<WorkspaceSettingsDto | null>(null);

  saveDraftRef.current = saveDraft;

  const drainSaveQueue = useCallback((): Promise<void> => {
    if (saveInFlightRef.current) return saveInFlightRef.current;
    const save = (async () => {
      await Promise.resolve();
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
        saveInFlightRef.current = null;
        frontendLogger.debug("Generation draft save queue finished");
      }
    })();
    saveInFlightRef.current = save;
    return save;
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
    if (!sourceReady || !settings) {
      return;
    }
    const sourceRevision = storedDraft?.revision ?? 0;
    const settingsChanged = hydratedSettingsRef.current !== settings;
    const newerExternalDraft = sourceRevision > revisionRef.current;
    if (!settingsChanged && !newerExternalDraft) return;
    if (pendingDraftRef.current || failedDraftRef.current || saveInFlightRef.current) return;
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
    revisionRef.current = sourceRevision;
    hydratedSettingsRef.current = settings;
    latestDraftRef.current = next;
    setDraft(next);
  }, [settings, sourceReady, storedDraft]);

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

  const flushAndWait = useCallback(async () => {
    if (saveTimerRef.current !== null) {
      window.clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }
    if (latestDraftRef.current && !pendingDraftRef.current && !saveInFlightRef.current) {
      pendingDraftRef.current = latestDraftRef.current;
    }
    await drainSaveQueue();
    if (failedDraftRef.current)
      throw new Error("Generation draft must be saved before starting the Agent.");
  }, [drainSaveQueue]);

  const flushDraft = useCallback(() => {
    reportBackgroundPromise(flushAndWait(), "Flush generation draft");
  }, [flushAndWait]);

  useEffect(() => {
    mountedRef.current = true;
    const unregister = registerGenerationDraftFlush(flushAndWait);
    return () => {
      mountedRef.current = false;
      unregister();
      if (saveTimerRef.current !== null) window.clearTimeout(saveTimerRef.current);
      if (pendingDraftRef.current || saveInFlightRef.current) {
        reportBackgroundPromise(
          trackGenerationDraftSave(flushAndWait()),
          "Persist generation draft on unmount",
        );
      }
    };
  }, [flushAndWait]);

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
