import { describe, expect, it } from "vitest";

import corpus from "../../../../../assets/prompt-syntax/corpus.json";
import { analyzePrompt, type NaiPromptProfile, type PromptSemanticSpan } from "./prompt-analysis";

describe("NAI prompt syntax corpus", () => {
  it.each(corpus.cases)("$name", (syntaxCase) => {
    const analysis = analyzePrompt(syntaxCase.text, parseProfile(syntaxCase.profile));
    expect(diagnosticCodes(analysis.diagnostics)).toEqual([...syntaxCase.diagnostics].sort());
  });

  it("keeps UTF-16 editor offsets stable for Unicode extension arguments", () => {
    const text = "🙂, $chunk(背景-简单背景), b";
    const functionSpan = analyzePrompt(text, "novelai_v45").semanticSpans.find(
      (span) => span.kind === "function",
    );
    expect(functionSpan).toBeDefined();
    expect(text.slice(functionSpan?.from, functionSpan?.to)).toBe("$chunk(背景-简单背景)");
    expect(functionSpan?.to).toBe(19);
  });

  it("recognizes compile-time comments and validates their arity", () => {
    expect(
      diagnosticCodes(analyzePrompt('$comment("draft note")', "novelai_v45").diagnostics),
    ).toEqual([]);
    expect(diagnosticCodes(analyzePrompt("$comment()", "novelai_v45").diagnostics)).toEqual([
      "invalid_function_arity",
    ]);
  });

  it("does not classify numeric-prefix tags as numeric weights", () => {
    expect(analyzePrompt("1girl", "novelai_v45")).toEqual({
      diagnostics: [],
      semanticSpans: [],
    });
  });

  it.each([
    ["{a}", [{ text: "a", direction: "up", tier: 1 }]],
    ["[a]", [{ text: "a", direction: "down", tier: 1 }]],
    ["{{a}}", [{ text: "a", direction: "up", tier: 1 }]],
    ["[[[[a]]]]", [{ text: "a", direction: "down", tier: 1 }]],
  ] as const)("applies structural weights for %s", (text, expected) => {
    expect(weightedContent(text)).toEqual(expected);
  });

  it("clears every active weight at a numeric closing reset", () => {
    const text = "{a, 2::b::c}";
    expect(weightedContent(text)).toEqual([
      { text: "a", direction: "up", tier: 1 },
      { text: "b", direction: "up", tier: 2 },
    ]);
    expect(resetText(text)).toEqual(["::"]);
  });

  it("clears structural weights at a standalone reset without restoring them on close", () => {
    expect(weightedContent("{a::b}")).toEqual([{ text: "a", direction: "up", tier: 1 }]);
    expect(weightedContent("{[2::a::b]}c")).toEqual([{ text: "a", direction: "up", tier: 2 }]);
  });

  it("keeps new structural weights independent of cleared outer structures", () => {
    expect(weightedContent("{a::{b}c}")).toEqual([
      { text: "a", direction: "up", tier: 1 },
      { text: "b", direction: "up", tier: 1 },
    ]);
  });

  it("does not leak weight operators out of extension calls", () => {
    expect(weightedContent("$chunk({2::hero::}), neutral")).toEqual([]);
  });

  it("accepts a closed randomizer and diagnoses all malformed randomizer options", () => {
    expect(diagnosticCodes(analyzePrompt("||red|blue||", "novelai_v45").diagnostics)).toEqual([]);
    expect(diagnosticCodes(analyzePrompt("||red| |blue||", "novelai_v45").diagnostics)).toEqual([
      "empty_randomizer_option",
    ]);
  });
});

function diagnosticCodes(diagnostics: ReturnType<typeof analyzePrompt>["diagnostics"]): string[] {
  return diagnostics.flatMap((diagnostic) => (diagnostic.source ? [diagnostic.source] : [])).sort();
}

function weightedContent(text: string) {
  return analyzePrompt(text, "novelai_v45").semanticSpans.flatMap((span) =>
    span.kind === "weight" && span.role === "content"
      ? [{ text: text.slice(span.from, span.to), direction: span.direction, tier: span.tier }]
      : [],
  );
}

function resetText(text: string): string[] {
  return analyzePrompt(text, "novelai_v45")
    .semanticSpans.filter(
      (span): span is Extract<PromptSemanticSpan, { kind: "weight_reset" }> =>
        span.kind === "weight_reset",
    )
    .map((span) => text.slice(span.from, span.to));
}

function parseProfile(value: string): NaiPromptProfile {
  if (value === "novelai_v3" || value === "novelai_v4") return value;
  return "novelai_v45";
}
