import { describe, expect, it } from "vitest";

import corpus from "../../../../../assets/prompt-syntax/corpus.json";
import {
  analyzePrompt,
  parsePrompt,
  tokenizePrompt,
  type NaiPromptProfile,
} from "./prompt-analysis";

describe("NAI prompt syntax corpus", () => {
  it.each(corpus.cases)("$name", (syntaxCase) => {
    const analysis = analyzePrompt(syntaxCase.text, parseProfile(syntaxCase.profile));
    const codes = analysis.diagnostics.map((diagnostic) => diagnostic.source);
    for (const expected of syntaxCase.diagnostics) expect(codes).toContain(expected);
    expect(analysis.tokens.map((token) => token.text).join("")).toBe(syntaxCase.text);
  });

  it("keeps UTF-16 editor offsets stable for Unicode extension arguments", () => {
    const text = "a, $chunk(背景-简单背景), b";
    const tokens = tokenizePrompt(text);
    expect(tokens.at(-1)?.to).toBe(text.length);
    expect(tokens.map((token) => token.text).join("")).toBe(text);
  });

  it("does not classify numeric-prefix tags as numeric weights", () => {
    const tokens = tokenizePrompt("1girl, 1.5::cinematic::");
    expect(tokens[0]).toMatchObject({ kind: "tag", text: "1girl" });
    expect(analyzePrompt("1girl", "novelai_v45").diagnostics).toEqual([]);
  });

  it("uses one semantic model for weight operators, affected tags, and reset operators", () => {
    const text = "{{dagger}}, 3:: 1girl, ::, [[girl_cafe_gun]],";
    const parse = parsePrompt(text);
    const spans = parse.semanticSpans.map((span) => ({
      ...span,
      text: text.slice(span.from, span.to),
    }));

    expect(parse.tokens.find((token) => token.text === "1girl")?.kind).toBe("tag");
    expect(
      spans
        .filter((span) => span.kind === "weight" && span.role === "operator")
        .map((span) => span.text),
    ).toEqual(["{", "{", "}", "}", "3::", "[", "[", "]", "]"]);
    expect(
      spans.flatMap((span) =>
        span.kind === "weight" && span.role === "content"
          ? [{ text: span.text, direction: span.direction, tier: span.tier }]
          : [],
      ),
    ).toEqual([
      { text: "dagger", direction: "up", tier: 1 },
      { text: "1girl", direction: "up", tier: 3 },
      { text: "girl_cafe_gun", direction: "down", tier: 1 },
    ]);
    expect(spans.filter((span) => span.kind === "weight_reset").map((span) => span.text)).toEqual([
      "::",
    ]);
  });
});

function parseProfile(value: string): NaiPromptProfile {
  if (value === "novelai_v3" || value === "novelai_v4") return value;
  return "novelai_v45";
}
