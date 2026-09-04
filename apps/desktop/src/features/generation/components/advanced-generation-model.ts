import type { CharacterReferenceTypeDto, PromptTokenUsageDto } from "@/types";

import {
  isGenerationCharacterEligible,
  type GenerationCharacterDraft,
  type GenerationDraft,
} from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";

export const REFERENCE_TYPE_OPTIONS = [
  { value: "character", label: "Character" },
  { value: "style", label: "Style" },
  { value: "character_and_style", label: "Character + style" },
] as const;

export function isCharacterReferenceType(value: string): value is CharacterReferenceTypeDto {
  return REFERENCE_TYPE_OPTIONS.some((option) => option.value === value);
}

export function patchPreciseReference(
  draft: GenerationDraft,
  onPatch: (patch: Partial<GenerationDraft>) => void,
  id: string,
  patch: Partial<GenerationDraft["preciseReferences"][number]>,
) {
  onPatch({
    preciseReferences: draft.preciseReferences.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    ),
  });
}

export function patchCharacter(
  draft: GenerationDraft,
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void,
  id: string,
  patch: Partial<GenerationDraft["characters"][number]>,
  options?: GenerationDraftPatchOptions,
) {
  onPatch(
    {
      characters: draft.characters.map((item) => (item.id === id ? { ...item, ...patch } : item)),
    },
    options,
  );
}

export function characterTokenCount(
  usage: PromptTokenUsageDto | null,
  characters: ReadonlyArray<GenerationCharacterDraft>,
  index: number,
  promptType: "positive" | "negative",
) {
  if (!usage) return null;
  const current = characters[index];
  if (!current || !isGenerationCharacterEligible(current)) {
    return { used: 0, limit: usage.prompt.limit };
  }
  const effectiveIndex = characters.slice(0, index).filter(isGenerationCharacterEligible).length;
  const character = usage.characters.find((item) => item.index === effectiveIndex);
  if (!character) return null;
  return promptType === "positive" ? character.prompt : character.negative_prompt;
}

export function createLocalId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Math.random().toString(16).slice(2)}`;
}
