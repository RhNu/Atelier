import { describe, expect, it } from "vitest";

import {
  buildPromptCompletionEdit,
  getPromptCompletionContext,
  type PromptCompletionItem,
} from "./completion-model";

const tag: PromptCompletionItem = {
  kind: "tag",
  id: "tag:cinematic_lighting",
  label: "cinematic_lighting",
  value: "cinematic_lighting",
  detail: "cinematic lighting",
  rank: "prefix",
};
const chunk: PromptCompletionItem = {
  kind: "chunk",
  id: "chunk:lighting",
  label: "lighting",
  value: "lighting",
  detail: "dramatic light",
  rank: "prefix",
};

describe("prompt completion model", () => {
  it("describes both the filter and full-token replacement ranges", () => {
    expect(getPromptCompletionContext("1girl, cinematic, solo", 11)).toEqual({
      mode: "tag",
      query: "cine",
      filterStart: 7,
      filterEnd: 11,
      replaceStart: 7,
      replaceEnd: 16,
      manual: false,
    });
  });

  it.each([
    ["cine", 4, false, "cinematic_lighting, ", 20],
    ["cine, solo", 2, false, "cinematic_lighting, solo", 20],
    ["cine,solo", 2, false, "cinematic_lighting, solo", 20],
    ["cine   ,   solo", 2, false, "cinematic_lighting, solo", 20],
    ["cine\nsolo", 2, false, "cinematic_lighting\nsolo", 18],
    ["cine|solo", 2, false, "cinematic_lighting|solo", 18],
    ["{cine}", 3, false, "{cinematic_lighting, }", 21],
    ["1girl, solo", 7, true, "1girl, cinematic_lighting, solo", 27],
    ["1girl,  solo", 7, true, "1girl, cinematic_lighting, solo", 27],
    ["🙂, ci", 6, false, "🙂, cinematic_lighting, ", 24],
  ] as const)(
    "applies a tag completion to %s at %s",
    (value, caret, manual, expectedValue, expectedCaret) => {
      const edit = buildPromptCompletionEdit(value, caret, tag, manual);
      expect(edit.value).toBe(expectedValue);
      expect(edit.selectionStart).toBe(expectedCaret);
      expect(edit.selectionEnd).toBe(expectedCaret);
    },
  );

  it("completes a function name and places the caret inside the call", () => {
    expect(
      buildPromptCompletionEdit("$ch", 3, { ...chunk, kind: "function", value: "chunk" }),
    ).toMatchObject({ value: "$chunk(", selectionStart: 7, selectionEnd: 7 });
  });

  it("inserts a function call when manually completed from an empty tag slot", () => {
    expect(
      buildPromptCompletionEdit("1girl, ", 7, { ...chunk, kind: "function", value: "chunk" }, true),
    ).toMatchObject({ value: "1girl, $chunk(", selectionStart: 14, selectionEnd: 14 });
  });

  it("replaces a complete chunk argument and consumes its closing parenthesis", () => {
    expect(buildPromptCompletionEdit("$chunk(li), solo", 9, chunk)).toMatchObject({
      value: "$chunk(lighting), solo",
      selectionStart: 18,
      selectionEnd: 18,
    });
  });
});
