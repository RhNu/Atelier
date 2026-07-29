import {
  promptFunctionDefinition,
  promptFunctionParameter,
  type PromptArgumentCompletion,
  type PromptFunctionDefinition,
} from "./prompt-functions";

export type FunctionArgumentContext = {
  completion: PromptArgumentCompletion;
  query: string;
  queryStart: number;
};

type ActiveFunctionCall = { name: string; argumentStart: number };

export function functionArgumentAtCaret(
  beforeCaret: string,
  functions: readonly PromptFunctionDefinition[],
): FunctionArgumentContext | null {
  const call = activeFunctionCall(beforeCaret);
  if (!call) return null;
  const definition = promptFunctionDefinition(call.name, functions);
  if (!definition) return noArgumentCompletion(beforeCaret.length);

  const slot = currentArgumentSlot(beforeCaret, call.argumentStart);
  const parameter = promptFunctionParameter(definition, slot.index, slot.named);
  if (!parameter || !isArgumentQuery(slot.query)) {
    return noArgumentCompletion(beforeCaret.length);
  }
  return {
    completion: parameter.completion,
    query: slot.query,
    queryStart: slot.queryStart,
  };
}

function noArgumentCompletion(queryStart: number): FunctionArgumentContext {
  return { completion: null, query: "", queryStart };
}

function activeFunctionCall(value: string): ActiveFunctionCall | null {
  const calls: ActiveFunctionCall[] = [];
  let quoted = false;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index] ?? "";
    if (character === "\\") {
      index += 1;
      continue;
    }
    if (character === '"') {
      quoted = !quoted;
      continue;
    }
    if (quoted) continue;
    if (character === "$") {
      const match = /^\$([A-Za-z][A-Za-z0-9_-]*)\(/u.exec(value.slice(index));
      if (match?.[1] !== undefined) {
        calls.push({ name: match[1], argumentStart: index + match[0].length });
        index += match[0].length - 1;
      }
    } else if (character === ")") {
      calls.pop();
    }
  }
  return calls.at(-1) ?? null;
}

function currentArgumentSlot(value: string, argumentStart: number) {
  let index = 0;
  let slotStart = argumentStart;
  let quoted = false;
  for (let cursor = argumentStart; cursor < value.length; cursor += 1) {
    const character = value[cursor] ?? "";
    if (character === "\\") {
      cursor += 1;
      continue;
    }
    if (character === '"') {
      quoted = !quoted;
    } else if (!quoted && character === ",") {
      index += 1;
      slotStart = cursor + 1;
    }
  }

  const contentStart = skipLeadingWhitespace(value, slotStart);
  const equals = findUnquotedEquals(value, contentStart);
  const named =
    equals < 0
      ? null
      : /^[A-Za-z][A-Za-z0-9_-]*$/u.test(value.slice(contentStart, equals).trim())
        ? value.slice(contentStart, equals).trim()
        : null;
  const queryStart = skipLeadingWhitespace(value, equals < 0 ? contentStart : equals + 1);
  return { index, named, query: value.slice(queryStart), queryStart };
}

function findUnquotedEquals(value: string, start: number): number {
  let quoted = false;
  for (let index = start; index < value.length; index += 1) {
    const character = value[index] ?? "";
    if (character === "\\") {
      index += 1;
    } else if (character === '"') {
      quoted = !quoted;
    } else if (!quoted && character === "=") {
      return index;
    }
  }
  return -1;
}

function isArgumentQuery(value: string): boolean {
  return !/[,()[\]{}|\s"=]/u.test(value);
}

function skipLeadingWhitespace(value: string, start: number): number {
  let index = start;
  while (value[index] === " " || value[index] === "\t") index += 1;
  return index;
}
