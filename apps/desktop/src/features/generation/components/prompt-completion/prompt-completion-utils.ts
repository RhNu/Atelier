export type PromptCompletionMode = "tag" | "chunk";

export type PromptCompletionItem = {
  kind: PromptCompletionMode;
  id: string;
  label: string;
  value: string;
  detail: string | null;
  rank: string;
};

export type PromptCompletionContext = {
  mode: PromptCompletionMode;
  query: string;
  replaceStart: number;
  replaceEnd: number;
  manual: boolean;
};

export type PromptCompletionEdit = {
  value: string;
  selectionStart: number;
  selectionEnd: number;
};

const CHUNK_CALL_PATTERN = /@chunk\(([^,()[\]{}|\s]*)$/u;

export function getPromptCompletionContext(
  value: string,
  selectionStart: number,
  manual = false,
): PromptCompletionContext {
  const caret = clampCaret(value, selectionStart);
  const beforeCaret = value.slice(0, caret);
  const chunkMatch = CHUNK_CALL_PATTERN.exec(beforeCaret);

  if (chunkMatch?.[1] !== undefined) {
    return {
      mode: "chunk",
      query: chunkMatch[1],
      replaceStart: caret - chunkMatch[1].length,
      replaceEnd: value.at(caret) === ")" ? caret + 1 : caret,
      manual,
    };
  }

  const replaceStart = findTagTokenStart(beforeCaret);
  return {
    mode: "tag",
    query: value.slice(replaceStart, caret).trimStart(),
    replaceStart,
    replaceEnd: caret,
    manual,
  };
}

export function applyPromptCompletion(
  value: string,
  selectionStart: number,
  item: PromptCompletionItem,
): PromptCompletionEdit {
  const context = getPromptCompletionContext(value, selectionStart);
  const replacement = replacementTextForContext(context, item);
  const nextValue =
    value.slice(0, context.replaceStart) + replacement + value.slice(context.replaceEnd);
  const insertedEnd = context.replaceStart + replacement.length;
  const { suffix, selectionOffset } = smartSeparator(nextValue, insertedEnd);
  const finalValue = nextValue.slice(0, insertedEnd) + suffix + nextValue.slice(insertedEnd);
  const caret = insertedEnd + selectionOffset;

  return {
    value: finalValue,
    selectionStart: caret,
    selectionEnd: caret,
  };
}

function replacementTextForContext(
  context: PromptCompletionContext,
  item: PromptCompletionItem,
): string {
  if (item.kind === "tag") {
    return item.value;
  }

  if (context.mode === "chunk") {
    return `${item.value})`;
  }

  return `@chunk(${item.value})`;
}

function smartSeparator(
  value: string,
  insertedEnd: number,
): { suffix: string; selectionOffset: number } {
  const next = value.at(insertedEnd);

  if (next === "," || next === "\n" || next === "\r") {
    return { suffix: "", selectionOffset: 0 };
  }

  if (next === " " && value.at(insertedEnd + 1) === ",") {
    return { suffix: "", selectionOffset: 0 };
  }

  if (next === undefined || next === "") {
    return { suffix: ", ", selectionOffset: 2 };
  }

  if (next === " ") {
    return { suffix: ",", selectionOffset: 1 };
  }

  return { suffix: ", ", selectionOffset: 2 };
}

function findTagTokenStart(value: string): number {
  for (let index = value.length - 1; index >= 0; index -= 1) {
    if (isTagSeparator(value[index] ?? "")) {
      return skipLeadingWhitespace(value, index + 1);
    }
  }

  return skipLeadingWhitespace(value, 0);
}

function isTagSeparator(character: string): boolean {
  return character === "," || character === "\n" || character === "\r" || character === "|";
}

function clampCaret(value: string, selectionStart: number): number {
  return Math.min(value.length, Math.max(0, selectionStart));
}

function skipLeadingWhitespace(value: string, start: number): number {
  let index = start;
  while (value[index] === " " || value[index] === "\t") {
    index += 1;
  }
  return index;
}
