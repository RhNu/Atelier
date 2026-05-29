import { useCallback, useEffect, useState } from "react";

import type { WorkspaceSettingsDto } from "../../../types";
import { createGenerationDraft, type GenerationDraft } from "../model/generation-draft";

export function useGenerationDraft(settings: WorkspaceSettingsDto | undefined) {
  const [draft, setDraft] = useState<GenerationDraft | null>(null);

  useEffect(() => {
    if (!settings || draft) {
      return;
    }

    setDraft(createGenerationDraft(settings));
  }, [draft, settings]);

  const patchDraft = useCallback((patch: Partial<GenerationDraft>) => {
    setDraft((current) => (current ? { ...current, ...patch } : current));
  }, []);

  const patchSize = useCallback((patch: Partial<GenerationDraft["size"]>) => {
    setDraft((current) =>
      current ? { ...current, size: { ...current.size, ...patch } } : current,
    );
  }, []);

  return {
    draft,
    patchDraft,
    patchSize,
  };
}
