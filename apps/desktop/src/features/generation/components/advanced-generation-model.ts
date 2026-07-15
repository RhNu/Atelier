import type { CharacterReferenceTypeDto } from "@/types";

import type { GenerationDraft } from "../model/generation-draft";

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
  onPatch: (patch: Partial<GenerationDraft>) => void,
  id: string,
  patch: Partial<GenerationDraft["characters"][number]>,
) {
  onPatch({
    characters: draft.characters.map((item) => (item.id === id ? { ...item, ...patch } : item)),
  });
}

export function createLocalId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Math.random().toString(16).slice(2)}`;
}
