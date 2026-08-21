import { syntaxTree } from "@codemirror/language";
import type { Diagnostic } from "@codemirror/lint";
import { EditorState, Facet } from "@codemirror/state";

import { naiPromptLanguage } from "./language";
import { buildPromptDiagnostics } from "./prompt-diagnostics";
import { buildPromptSemanticSpans } from "./prompt-semantics";
import { inspectPromptTree } from "./prompt-syntax-tree";

export type NaiPromptProfile = "novelai_v3" | "novelai_v4" | "novelai_v45" | "novelai_v5";
export type PromptWeightDirection = "up" | "down" | "neutral";
export type PromptSemanticSpan =
  | {
      kind: "function";
      appearance: "default" | "comment";
      from: number;
      to: number;
    }
  | { kind: "weight_reset"; from: number; to: number }
  | {
      kind: "weight";
      role: "content" | "operator";
      direction: PromptWeightDirection;
      tier: number;
      from: number;
      to: number;
    };

export type PromptEditorMessages = {
  unmatchedStrengtheningClose: string;
  unmatchedWeakeningClose: string;
  unclosedStrengthening: string;
  unclosedWeakening: string;
  invalidNumericWeight: string;
  unsupportedNumericWeight: string;
  unsupportedNegativeNumericWeight: string;
  unclosedNumericWeight: string;
  unterminatedString: string;
  unclosedRandomizer: string;
  emptyRandomizerOption: string;
  unclosedFunctionCall: string;
  unknownFunction: string;
  invalidFunctionArity: string;
  invalidFunctionArgument: string;
};

export type PromptAnalysis = {
  diagnostics: Diagnostic[];
  semanticSpans: PromptSemanticSpan[];
};

const DEFAULT_MESSAGES: PromptEditorMessages = {
  unmatchedStrengtheningClose: "Strengthening close delimiter has no matching opener.",
  unmatchedWeakeningClose: "Weakening close delimiter has no matching opener.",
  unclosedStrengthening: "Strengthening block is not closed.",
  unclosedWeakening: "Weakening block is not closed.",
  invalidNumericWeight: "Numeric emphasis weight is invalid.",
  unsupportedNumericWeight: "Numeric emphasis is not supported by this model.",
  unsupportedNegativeNumericWeight: "Negative numeric emphasis requires NAI Diffusion 4.5.",
  unclosedNumericWeight: "Numeric emphasis remains active until the end of the prompt.",
  unterminatedString: "String literal is not closed.",
  unclosedRandomizer: "Prompt randomizer is not closed.",
  emptyRandomizerOption: "Prompt randomizer contains an empty option.",
  unclosedFunctionCall: "Extension function call is not closed.",
  unknownFunction: "Unknown prompt extension function.",
  invalidFunctionArity: "Prompt extension function has an invalid number of arguments.",
  invalidFunctionArgument: "Prompt extension function has an invalid named argument.",
};
const analysisCache = new WeakMap<EditorState, PromptAnalysis>();

export const naiPromptProfileFacet = Facet.define<NaiPromptProfile, NaiPromptProfile>({
  combine: (values) => values.at(-1) ?? "novelai_v45",
});
export const naiPromptMessagesFacet = Facet.define<PromptEditorMessages, PromptEditorMessages>({
  combine: (values) => values.at(-1) ?? DEFAULT_MESSAGES,
});

export function promptProfileForModel(model: string): NaiPromptProfile {
  switch (model) {
    case "nai-diffusion-5-full":
    case "nai-diffusion-5-curated":
      return "novelai_v5";
    case "nai-diffusion-4-5-full":
    case "nai-diffusion-4-5-curated":
      return "novelai_v45";
    case "nai-diffusion-4-full":
    case "nai-diffusion-4-curated":
      return "novelai_v4";
    default:
      return "novelai_v3";
  }
}

export function analyzePrompt(
  text: string,
  profile: NaiPromptProfile,
  messages: PromptEditorMessages = DEFAULT_MESSAGES,
): PromptAnalysis {
  return analyzeSyntax(
    text,
    inspectPromptTree(naiPromptLanguage.parser.parse(text)),
    profile,
    messages,
  );
}

export function promptAnalysisForState(state: EditorState): PromptAnalysis {
  const cached = analysisCache.get(state);
  if (cached) return cached;
  const analysis = analyzeSyntax(
    state.doc.toString(),
    inspectPromptTree(syntaxTree(state)),
    state.facet(naiPromptProfileFacet),
    state.facet(naiPromptMessagesFacet),
  );
  analysisCache.set(state, analysis);
  return analysis;
}

function analyzeSyntax(
  text: string,
  syntax: ReturnType<typeof inspectPromptTree>,
  profile: NaiPromptProfile,
  messages: PromptEditorMessages,
): PromptAnalysis {
  return {
    diagnostics: buildPromptDiagnostics(text, syntax, profile, messages),
    semanticSpans: buildPromptSemanticSpans(text, syntax),
  };
}
