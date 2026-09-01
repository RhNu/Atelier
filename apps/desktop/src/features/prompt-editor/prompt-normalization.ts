const FULL_WIDTH_PUNCTUATION = /[，；。．]/g;
const FULL_WIDTH_PUNCTUATION_CHARACTERS = "，；。．";

export type PromptNormalizationChange = {
  from: number;
  to: number;
  insert: string;
};

export function normalizeFullWidthPunctuation(value: string): string {
  return value.replace(FULL_WIDTH_PUNCTUATION, ", ");
}

export function fullWidthPunctuationChanges(value: string): PromptNormalizationChange[] {
  const changes: PromptNormalizationChange[] = [];
  for (let index = 0; index < value.length; index += 1) {
    if (FULL_WIDTH_PUNCTUATION_CHARACTERS.includes(value[index] ?? "")) {
      changes.push({ from: index, to: index + 1, insert: ", " });
    }
  }
  return changes;
}
