import { useCallback, useEffect, useRef, useState } from "react";

import type { GenerationDraftDto, WorkspaceSettingsDto } from "../../../types";
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
  storedDraft: GenerationDraftDto | null | undefined;
  sourceReady: boolean;
  saveDraft: (draft: GenerationDraft) => Promise<unknown>;
};

const SAVE_DEBOUNCE_MS = 250;

export function useGenerationDraft({
  settings,
  storedDraft,
  sourceReady,
  saveDraft,
}: UseGenerationDraftOptions) {
  const [draft, setDraft] = useState<GenerationDraft | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const latestDraftRef = useRef<GenerationDraft | null>(null);
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
    try {
      while (pendingDraftRef.current) {
        const next = pendingDraftRef.current;
        pendingDraftRef.current = null;
        try {
          await saveDraftRef.current(next);
          failedDraftRef.current = null;
          if (mountedRef.current) {
            setSaveError(null);
          }
        } catch (error) {
          failedDraftRef.current = next;
          if (mountedRef.current) {
            setSaveError(formatSaveError(error));
          }
          break;
        }
      }
    } finally {
      saveInFlightRef.current = false;
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
        void drainSaveQueue();
        return;
      }
      saveTimerRef.current = window.setTimeout(() => {
        saveTimerRef.current = null;
        void drainSaveQueue();
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
      ? generationDraftFromDto(storedDraft)
      : createGenerationDraft(settings);
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
        void saveDraftRef.current(pending);
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

  return {
    draft,
    patchDraft,
    patchSize,
    replaceDraft,
    flushDraft,
    retrySave,
    saveError,
  };
}

function formatSaveError(error: unknown): string {
  return error instanceof Error ? error.message : "Generation draft could not be saved.";
}
