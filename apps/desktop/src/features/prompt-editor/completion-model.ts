import { functionArgumentAtCaret } from "./function-argument-completion";
import { NAI_PROMPT_FUNCTIONS, type PromptFunctionDefinition } from "./prompt-functions";

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
  filterStart: number;
  filterEnd: number;
  replaceStart: number;
  replaceEnd: number;
  manual: boolean;
};

export type PromptCompletionEdit = {
  value: string;
  selectionStart: number;
  selectionEnd: number;
  replaceStart: number;
  replaceEnd: number;
  insert: string;
};

const FUNCTION_CALL_PATTERN = /\$([A-Za-z0-9_-]*)$/u;

export function getPromptCompletionContext(
  value: string,
  selectionStart: number,
  manual = false,
  functions: readonly PromptFunctionDefinition[] = NAI_PROMPT_FUNCTIONS,
): PromptCompletionContext {
  const caret = clampCaret(value, selectionStart);
  const beforeCaret = value.slice(0, caret);
  const argument = functionArgumentAtCaret(beforeCaret, functions);

  if (argument?.completion === "chunk") {
    return {
      mode: "chunk",
      query: argument.query,
      filterStart: argument.queryStart,
      filterEnd: caret,
      replaceStart: argument.queryStart,
      replaceEnd: findArgumentReplaceEnd(value, caret),
      manual,
    };
  }

  const functionMatch = FUNCTION_CALL_PATTERN.exec(beforeCaret);
  if (functionMatch?.[1] !== undefined) {
    const start = caret - functionMatch[1].length;
    return {
      mode: "function",
      query: functionMatch[1],
      filterStart: start,
      filterEnd: caret,
      replaceStart: start,
      replaceEnd: findFunctionReplaceEnd(value, caret),
      manual,
    };
  }

  if (argument !== null) {
    return emptyTagContext(caret, manual);
  }

  const replaceStart = findTagTokenStart(beforeCaret);
  const query = value.slice(replaceStart, caret).trimStart();
  const insertAtEmptySlot = manual && query.length === 0;
  return {
    mode: "tag",
    query,
    filterStart: replaceStart,
    filterEnd: caret,
    replaceStart,
    replaceEnd: insertAtEmptySlot ? caret : findTagTokenEnd(value, caret),
    manual,
  };
}

export function buildPromptCompletionEdit(
  value: string,
  selectionStart: number,
  item: PromptCompletionItem,
  manual = false,
): PromptCompletionEdit {
  const context = getPromptCompletionContext(value, selectionStart, manual);
  const replacement = replacementTextForContext(context, item);
  if (context.mode === "function" || item.kind === "function") {
    return buildEdit(
      value,
      context.replaceStart,
      context.replaceEnd,
      replacement,
      replacement.length,
    );
  }

  const separator = separatorEdit(value, context.replaceEnd);
  const insert = replacement + separator.text;
  return buildEdit(
    value,
    context.replaceStart,
    separator.replaceEnd,
    insert,
    replacement.length + separator.caretOffset,
  );
}

function emptyTagContext(caret: number, manual: boolean): PromptCompletionContext {
  return {
    mode: "tag",
    query: "",
    filterStart: caret,
    filterEnd: caret,
    replaceStart: caret,
    replaceEnd: caret,
    manual,
  };
}

function buildEdit(
  value: string,
  replaceStart: number,
  replaceEnd: number,
  insert: string,
  selectionOffset: number,
): PromptCompletionEdit {
  const selectionStart = replaceStart + selectionOffset;
  return {
    value: value.slice(0, replaceStart) + insert + value.slice(replaceEnd),
    selectionStart,
    selectionEnd: selectionStart,
    replaceStart,
    replaceEnd,
    insert,
  };
}

function replacementTextForContext(
  context: PromptCompletionContext,
  item: PromptCompletionItem,
): string {
  if (item.kind === "tag") return item.value;
  if (item.kind === "function") {
    return context.mode === "function" ? `${item.value}(` : `$${item.value}(`;
  }
  if (context.mode === "chunk") return `${item.value})`;
  return `$chunk(${item.value})`;
}

function separatorEdit(
  value: string,
  insertedEnd: number,
): { text: string; caretOffset: number; replaceEnd: number } {
  let whitespaceEnd = insertedEnd;
  while (isHorizontalWhitespace(value[whitespaceEnd])) whitespaceEnd += 1;

  const next = value[whitespaceEnd];
  if (next === "|") {
    return { text: "", caretOffset: 0, replaceEnd: insertedEnd };
  }

  if (next === ",") {
    let trailingEnd = whitespaceEnd + 1;
    while (isHorizontalWhitespace(value[trailingEnd])) trailingEnd += 1;
    const suffix = isLineBreak(value[trailingEnd]) ? "," : ", ";
    return { text: suffix, caretOffset: suffix.length, replaceEnd: trailingEnd };
  }

  if (isLineBreak(next)) {
    return { text: ",", caretOffset: 1, replaceEnd: whitespaceEnd };
  }

  return { text: ", ", caretOffset: 2, replaceEnd: whitespaceEnd };
}

function findTagTokenStart(value: string): number {
  for (let index = value.length - 1; index >= 0; index -= 1) {
    if (isTagTokenBoundary(value[index] ?? "")) return skipLeadingWhitespace(value, index + 1);
  }
  return skipLeadingWhitespace(value, 0);
}

function findTagTokenEnd(value: string, start: number): number {
  let index = start;
  while (index < value.length && !isTagTokenBoundary(value[index] ?? "")) index += 1;
  return trimTrailingWhitespace(value, start, index);
}

function findFunctionReplaceEnd(value: string, start: number): number {
  let index = start;
  while (/[A-Za-z0-9_-]/u.test(value[index] ?? "")) index += 1;
  return value[index] === "(" ? index + 1 : index;
}

function findArgumentReplaceEnd(value: string, start: number): number {
  let index = start;
  while (index < value.length && !/[,()[\]{}|\s]/u.test(value[index] ?? "")) index += 1;
  return value[index] === ")" ? index + 1 : index;
}

function isTagTokenBoundary(character: string): boolean {
  return ",\n\r|{}[]".includes(character);
}

function isHorizontalWhitespace(character: string | undefined): boolean {
  return character === " " || character === "\t";
}

function isLineBreak(character: string | undefined): boolean {
  return character === "\n" || character === "\r";
}

function clampCaret(value: string, selectionStart: number): number {
  return Math.min(value.length, Math.max(0, selectionStart));
}

function skipLeadingWhitespace(value: string, start: number): number {
  let index = start;
  while (isHorizontalWhitespace(value[index])) index += 1;
  return index;
}

function trimTrailingWhitespace(value: string, minimum: number, end: number): number {
  let index = end;
  while (index > minimum && isHorizontalWhitespace(value[index - 1])) index -= 1;
  return index;
}
