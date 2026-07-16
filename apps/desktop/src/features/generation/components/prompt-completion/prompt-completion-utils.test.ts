import {
  applyPromptCompletion,
  getPromptCompletionContext,
  type PromptCompletionItem,
} from "./prompt-completion-utils";

const tagItem: PromptCompletionItem = {
  kind: "tag",
  id: "tag:cinematic_lighting",
  label: "cinematic_lighting",
  value: "cinematic_lighting",
  detail: "cinematic lighting",
  rank: "prefix",
};

const chunkItem: PromptCompletionItem = {
  kind: "chunk",
  id: "chunk:lighting",
  label: "lighting",
  value: "lighting",
  detail: "dramatic light",
  rank: "prefix",
};

describe("prompt completion utilities", () => {
  it("detects the current prompt token before the caret", () => {
    expect(getPromptCompletionContext("1girl, cine", 11)).toEqual({
      mode: "tag",
      query: "cine",
      replaceStart: 7,
      replaceEnd: 11,
      manual: false,
    });
  });

  it("replaces a tag fragment and inserts a smart comma separator", () => {
    expect(applyPromptCompletion("1girl, cine", 11, tagItem)).toEqual({
      value: "1girl, cinematic_lighting, ",
      selectionStart: 27,
      selectionEnd: 27,
    });
  });

  it("replaces a chunk fragment inside $chunk(...) with a chunk call", () => {
    expect(applyPromptCompletion("1girl, $chunk(li", 16, chunkItem)).toEqual({
      value: "1girl, $chunk(lighting), ",
      selectionStart: 25,
      selectionEnd: 25,
    });
  });

  it("detects function names after a dollar and places the caret inside the call", () => {
    expect(getPromptCompletionContext("$ch", 3)).toEqual({
      mode: "function",
      query: "ch",
      replaceStart: 1,
      replaceEnd: 3,
      manual: false,
    });
    expect(
      applyPromptCompletion("$ch", 3, { ...chunkItem, kind: "function", value: "chunk" }),
    ).toEqual({
      value: "$chunk(",
      selectionStart: 7,
      selectionEnd: 7,
    });
  });

  it("consumes an existing chunk closing parenthesis before adding a separator", () => {
    expect(applyPromptCompletion("$chunk(li)", 9, chunkItem)).toEqual({
      value: "$chunk(lighting), ",
      selectionStart: 18,
      selectionEnd: 18,
    });
  });

  it("does not duplicate an existing comma or newline after an accepted item", () => {
    expect(applyPromptCompletion("cine, solo", 4, tagItem)).toEqual({
      value: "cinematic_lighting, solo",
      selectionStart: 18,
      selectionEnd: 18,
    });
    expect(applyPromptCompletion("cine\nsolo", 4, tagItem)).toEqual({
      value: "cinematic_lighting\nsolo",
      selectionStart: 18,
      selectionEnd: 18,
    });
    expect(applyPromptCompletion("cine|solo", 4, tagItem)).toEqual({
      value: "cinematic_lighting|solo",
      selectionStart: 18,
      selectionEnd: 18,
    });
  });

  it("replaces the complete token when completion is accepted from the middle", () => {
    expect(applyPromptCompletion("1girl, cinematic, solo", 11, tagItem)).toEqual({
      value: "1girl, cinematic_lighting, solo",
      selectionStart: 25,
      selectionEnd: 25,
    });
    expect(applyPromptCompletion("1girl, $chunk(lighting), solo", 16, chunkItem)).toEqual({
      value: "1girl, $chunk(lighting), solo",
      selectionStart: 23,
      selectionEnd: 23,
    });
    expect(
      applyPromptCompletion("$chunk(", 3, { ...chunkItem, kind: "function", value: "chunk" }),
    ).toEqual({
      value: "$chunk(",
      selectionStart: 7,
      selectionEnd: 7,
    });
  });
});
