import type { PromptPresetBehaviorDto, PromptPresetDto } from "@/types";

export type AppliedPromptPreset = {
  prompt: string;
  negativePrompt: string;
};

export function applyPromptPreset(
  preset: PromptPresetDto,
  prompt: string,
  negativePrompt: string,
): AppliedPromptPreset {
  return {
    prompt: applyPromptBehavior(prompt, preset.prompt_behavior),
    negativePrompt: applyPromptBehavior(negativePrompt, preset.uc_behavior),
  };
}

export function applyPromptBehavior(base: string, behavior: PromptPresetBehaviorDto): string {
  if (behavior.mode === "replace") {
    return behavior.text;
  }

  return [behavior.before, base, behavior.after]
    .filter((fragment) => fragment.trim().length > 0)
    .reduce((output, fragment) => appendExpansion(output, fragment), "");
}

function appendExpansion(output: string, fragment: string): string {
  const left = significantTail(output);
  const right = significantHead(fragment);
  if (left === null) {
    return trimLeadingBoundary(fragment);
  }
  if (right === null) {
    return trimTrailingBoundary(output);
  }
  if (!canNormalizeBoundary(left, right)) {
    return output + fragment;
  }
  return `${trimTrailingBoundary(output)}, ${trimLeadingBoundary(fragment)}`;
}

function significantTail(value: string): string | null {
  return trimTrailingBoundary(value).at(-1) ?? null;
}

function significantHead(value: string): string | null {
  return trimLeadingBoundary(value).at(0) ?? null;
}

function canNormalizeBoundary(left: string, right: string): boolean {
  return !"{[(|:".includes(left) && !"}])|:".includes(right);
}

function trimTrailingBoundary(value: string): string {
  return value.replace(/[\s,]+$/u, "");
}

function trimLeadingBoundary(value: string): string {
  return value.replace(/^[\s,]+/u, "");
}
