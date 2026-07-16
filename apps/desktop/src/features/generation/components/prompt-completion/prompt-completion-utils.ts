export type PromptCompletionMode = "tag" | "function" | "chunk";

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

export function promptFunctionCompletionItems(
  context: PromptCompletionContext | null,
): PromptCompletionItem[] {
  if (!context || context.mode !== "function") return [];
  const query = context.query.toLowerCase().replaceAll("_", " ").trim();
  if (query.length > 0 && !"chunk".includes(query)) return [];
  return [
    {
      kind: "function",
      id: "function:chunk",
      label: "chunk",
      value: "chunk",
      detail: "Reusable prompt chunk",
      rank: "function",
    },
  ];
}

const FUNCTION_CALL_PATTERN = /\$([A-Za-z0-9_-]*)$/u;
const CHUNK_CALL_PATTERN = /\$chunk\(([^,()[\]{}|\s]*)$/u;

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
      replaceEnd: findChunkReplaceEnd(value, caret),
      manual,
    };
  }

  const functionMatch = FUNCTION_CALL_PATTERN.exec(beforeCaret);
  if (functionMatch?.[1] !== undefined) {
    return {
      mode: "function",
      query: functionMatch[1],
      replaceStart: caret - functionMatch[1].length,
      replaceEnd: findFunctionReplaceEnd(value, caret),
      manual,
    };
  }

  if (/\$[A-Za-z0-9_-]+\([^)]*\)$/u.test(beforeCaret)) {
    return {
      mode: "tag",
      query: "",
      replaceStart: caret,
      replaceEnd: caret,
      manual,
    };
  }

  const replaceStart = findTagTokenStart(beforeCaret);
  return {
    mode: "tag",
    query: value.slice(replaceStart, caret).trimStart(),
    replaceStart,
    replaceEnd: findTagTokenEnd(value, caret),
    manual,
  };
}

export function applyPromptCompletion(
  value: string,
  selectionStart: number,
  item: PromptCompletionItem,
): PromptCompletionEdit {
  const edit = buildPromptCompletionEdit(value, selectionStart, item);
  return {
    value: edit.value,
    selectionStart: edit.selectionStart,
    selectionEnd: edit.selectionEnd,
  };
}

export function buildPromptCompletionEdit(
  value: string,
  selectionStart: number,
  item: PromptCompletionItem,
): PromptCompletionEdit & { replaceStart: number; replaceEnd: number; insert: string } {
  const context = getPromptCompletionContext(value, selectionStart);
  const replacement = replacementTextForContext(context, item);
  const nextValue =
    value.slice(0, context.replaceStart) + replacement + value.slice(context.replaceEnd);
  const insertedEnd = context.replaceStart + replacement.length;
  if (context.mode === "function") {
    return {
      value: nextValue,
      selectionStart: insertedEnd,
      selectionEnd: insertedEnd,
      replaceStart: context.replaceStart,
      replaceEnd: context.replaceEnd,
      insert: replacement,
    };
  }
  const { suffix, selectionOffset } = smartSeparator(nextValue, insertedEnd);
  const finalValue = nextValue.slice(0, insertedEnd) + suffix + nextValue.slice(insertedEnd);
  const caret = insertedEnd + selectionOffset;
  return {
    value: finalValue,
    selectionStart: caret,
    selectionEnd: caret,
    replaceStart: context.replaceStart,
    replaceEnd: context.replaceEnd,
    insert: replacement + suffix,
  };
}

function replacementTextForContext(
  context: PromptCompletionContext,
  item: PromptCompletionItem,
): string {
  if (item.kind === "tag") {
    return item.value;
  }

  if (context.mode === "function") {
    return `${item.value}(`;
  }

  if (context.mode === "chunk") {
    return `${item.value})`;
  }

  return `$chunk(${item.value})`;
}

function smartSeparator(
  value: string,
  insertedEnd: number,
): { suffix: string; selectionOffset: number } {
  const next = value.at(insertedEnd);

  if (next === "," || next === "\n" || next === "\r" || next === "|") {
    return { suffix: "", selectionOffset: 0 };
  }

  if (next === " " && value.at(insertedEnd + 1) === ",") {
    return { suffix: "", selectionOffset: 0 };
  }

  if (next === undefined || next === "") {
    return { suffix: ", ", selectionOffset: 2 };
  }

  if (next === " ") {
    if (value.at(insertedEnd + 1) === "|" || value.at(insertedEnd + 1) === "\n") {
      return { suffix: "", selectionOffset: 0 };
    }
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

function findTagTokenEnd(value: string, start: number): number {
  let index = start;
  while (index < value.length && !isTagSeparator(value[index] ?? "")) index += 1;
  return trimTrailingWhitespace(value, start, index);
}

function findFunctionReplaceEnd(value: string, start: number): number {
  let index = start;
  while (/[A-Za-z0-9_-]/u.test(value[index] ?? "")) index += 1;
  return value[index] === "(" ? index + 1 : index;
}

function findChunkReplaceEnd(value: string, start: number): number {
  let index = start;
  while (index < value.length && !/[,()[\]{}|\s]/u.test(value[index] ?? "")) index += 1;
  return value[index] === ")" ? index + 1 : index;
}

function isTagSeparator(character: string): boolean {
  return ",\n\r|{}[]".includes(character);
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

function trimTrailingWhitespace(value: string, minimum: number, end: number): number {
  let index = end;
  while (index > minimum && (value[index - 1] === " " || value[index - 1] === "\t")) index -= 1;
  return index;
}
