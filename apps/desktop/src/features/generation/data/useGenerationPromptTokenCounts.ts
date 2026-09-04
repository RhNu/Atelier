import { useEffect, useState } from "react";

import { generationApi } from "@/platform/atelier";
import type { CountPromptTokensRequestDto, PromptTokenUsageDto } from "@/types";

import { isGenerationCharacterEligible, type GenerationDraft } from "../model/generation-draft";

export function useGenerationPromptTokenCounts(
  draft: GenerationDraft | null,
): PromptTokenUsageDto | null {
  const [counts, setCounts] = useState<PromptTokenUsageDto | null>(null);

  useEffect(() => {
    if (!draft) return;
    let active = true;
    const request = promptTokenCountRequest(draft);
    async function refreshCount() {
      try {
        const next = await generationApi.countPromptTokens(request);
        if (active) setCounts(next);
      } catch {
        if (active) setCounts(null);
      }
    }
    const timer = window.setTimeout(() => {
      void refreshCount();
    }, 160);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [draft]);

  return draft ? counts : null;
}

export function promptTokenCountRequest(draft: GenerationDraft): CountPromptTokensRequestDto {
  return {
    compile: {
      model: draft.model,
      main_preset_id: draft.mainPresetId,
      prompt: draft.prompt,
      negative_prompt: draft.negativePrompt.trim() ? draft.negativePrompt : null,
      characters: draft.characters.filter(isGenerationCharacterEligible).map((character) => ({
        preset_id: character.presetId,
        prompt: character.prompt,
        negative_prompt: character.negativePrompt.trim() ? character.negativePrompt : null,
        enabled: true,
      })),
      max_depth: 16,
    },
    quality: draft.quality,
    transparent_background: draft.transparentBackground,
    uc_preset: draft.ucPreset,
    furry_mode: draft.furryMode,
  };
}
