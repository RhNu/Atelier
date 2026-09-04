import type { GenerationCharacterDraft } from "./generation-draft";

const FREEFORM_DEFAULTS = [
  [0.5, 0.5],
  [0.3, 0.5],
  [0.7, 0.5],
  [0.5, 0.3],
  [0.5, 0.7],
  [0.3, 0.3],
  [0.7, 0.3],
  [0.3, 0.7],
  [0.7, 0.7],
] as const;

export function initializePositionDraft(
  characters: ReadonlyArray<GenerationCharacterDraft>,
): GenerationCharacterDraft[] {
  const used = new Set<string>();
  return characters.map((character, index) => {
    const key = `${round3(character.position.x)}:${round3(character.position.y)}`;
    const fallback = FREEFORM_DEFAULTS[index % FREEFORM_DEFAULTS.length] ?? [0.5, 0.5];
    const position =
      used.has(key) && index > 0 ? { x: fallback[0], y: fallback[1] } : { ...character.position };
    used.add(`${round3(position.x)}:${round3(position.y)}`);
    return { ...character, position };
  });
}

function round3(value: number): number {
  return Math.round(value * 1000) / 1000;
}
