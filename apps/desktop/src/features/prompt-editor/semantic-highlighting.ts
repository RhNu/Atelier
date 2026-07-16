import { RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import { tokenizePrompt, type PromptToken } from "./prompt-analysis";

const UP_FACTOR = 1.05;
const DOWN_FACTOR = 1 / UP_FACTOR;
const MAX_TIER = 4;

const markCache = new Map<string, Decoration>();

export const naiSemanticHighlighting = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view.state.doc.toString());
    }

    update(update: ViewUpdate) {
      if (!update.docChanged) return;
      this.decorations = buildDecorations(update.state.doc.toString());
    }
  },
  { decorations: (value) => value.decorations },
);

function buildDecorations(text: string): DecorationSet {
  try {
    const tokens = tokenizePrompt(text);
    const functionRanges = findFunctionRanges(tokens);
    const numericOpens = findNumericOpens(tokens, functionRanges);
    const numericCloses = findNumericCloses(tokens, numericOpens, functionRanges);
    const effectiveWeights = new Map<number, number>();
    const delimiterClasses = new Map<number, string>();
    const closeAll = new Set<number>();
    const stack: Array<{ kind: "up" | "down" | "numeric"; factor: number }> = [];
    const builder = new RangeSetBuilder<Decoration>();

    for (let index = 0; index < tokens.length; index += 1) {
      const token = tokens[index];
      if (!token) continue;
      const insideFunction = functionRanges.some(([start, end]) => index >= start && index <= end);
      if (!insideFunction) {
        effectiveWeights.set(
          index,
          stack.reduce((value, item) => value * item.factor, 1),
        );
      }

      if (!insideFunction && numericOpens.has(index)) {
        stack.push({ kind: "numeric", factor: Number(token.text) });
        const next = tokens[index + 1];
        if (next) delimiterClasses.set(index + 1, directionClass(Number(token.text), true));
      } else if (!insideFunction && numericCloses.has(index)) {
        closeAll.add(index);
        popKind(stack, "numeric");
      } else if (!insideFunction && token.text === "{") {
        stack.push({ kind: "up", factor: UP_FACTOR });
        delimiterClasses.set(index, directionClass(UP_FACTOR, true));
      } else if (!insideFunction && token.text === "[") {
        stack.push({ kind: "down", factor: DOWN_FACTOR });
        delimiterClasses.set(index, directionClass(DOWN_FACTOR, true));
      } else if (!insideFunction && (token.text === "}" || token.text === "]")) {
        const kind = token.text === "}" ? "up" : "down";
        delimiterClasses.set(index, directionClass(kind === "up" ? UP_FACTOR : DOWN_FACTOR, true));
        popKind(stack, kind);
      } else if (!insideFunction && token.text === "::") {
        closeAll.add(index);
        stack.length = 0;
      }

      const classes = classesForToken(
        token,
        index,
        effectiveWeights.get(index) ?? 1,
        numericOpens,
        functionRanges,
        delimiterClasses,
        closeAll,
      );
      if (classes.length > 0) {
        builder.add(token.from, token.to, markFor(classes));
      }
    }
    return builder.finish();
  } catch (error) {
    console.error("[NaiPromptEditor] semantic highlighting disabled after an error", error);
    return Decoration.none;
  }
}

function classesForToken(
  token: PromptToken,
  index: number,
  weight: number,
  numericOpens: Set<number>,
  functionRanges: Array<[number, number]>,
  delimiterClasses: Map<number, string>,
  closeAll: Set<number>,
): string[] {
  const classes: string[] = [];
  if (functionRanges.some(([start, end]) => index >= start && index <= end)) {
    classes.push("nai-function");
  }
  const delimiter = delimiterClasses.get(index);
  if (delimiter) classes.push(delimiter);
  if (closeAll.has(index)) classes.push("nai-close-all");
  if (numericOpens.has(index)) {
    const numericWeight = Number(token.text);
    classes.push(
      Math.abs(numericWeight - 1) > 0.0001 ? directionClass(numericWeight, false) : "nai-number",
    );
  } else if (token.kind === "number" && Math.abs(weight - 1) > 0.0001) {
    classes.push(directionClass(weight, false));
  } else if (!delimiter && !closeAll.has(index) && Math.abs(weight - 1) > 0.0001) {
    classes.push(directionClass(weight, false));
  }
  return classes;
}

function findFunctionRanges(tokens: PromptToken[]): Array<[number, number]> {
  const ranges: Array<[number, number]> = [];
  for (let index = 0; index + 2 < tokens.length; index += 1) {
    if (
      tokens[index]?.text !== "$" ||
      tokens[index + 1]?.kind !== "identifier" ||
      tokens[index + 2]?.text !== "("
    ) {
      continue;
    }
    const close = findTokenIndex(tokens, index + 3, (token) => token.text === ")");
    ranges.push([index, close ?? tokens.length - 1]);
    index = close ?? tokens.length;
  }
  return ranges;
}

function findNumericOpens(
  tokens: PromptToken[],
  functionRanges: Array<[number, number]>,
): Set<number> {
  const opens = new Set<number>();
  for (let index = 0; index + 1 < tokens.length; index += 1) {
    if (functionRanges.some(([start, end]) => index >= start && index <= end)) continue;
    if (tokens[index]?.kind === "number" && tokens[index + 1]?.text === "::") opens.add(index);
  }
  return opens;
}

function findNumericCloses(
  tokens: PromptToken[],
  opens: Set<number>,
  functionRanges: Array<[number, number]>,
): Set<number> {
  const closes = new Set<number>();
  for (const open of opens) {
    const close = findTokenIndex(
      tokens,
      open + 2,
      (token, index) =>
        token.text === "::" &&
        !functionRanges.some(([start, end]) => index >= start && index <= end),
    );
    if (close !== undefined) closes.add(close);
  }
  return closes;
}

function findTokenIndex(
  tokens: PromptToken[],
  start: number,
  predicate: (token: PromptToken, index: number) => boolean,
): number | undefined {
  for (let index = start; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token && predicate(token, index)) return index;
  }
  return undefined;
}

function popKind(
  stack: Array<{ kind: "up" | "down" | "numeric"; factor: number }>,
  kind: "up" | "down" | "numeric",
) {
  const index = stack.findLastIndex((item) => item.kind === kind);
  if (index >= 0) stack.splice(index, 1);
}

function directionClass(weight: number, delimiter: boolean): string {
  const direction = weight > 1 ? "up" : "down";
  const tier = weightTier(weight);
  return `nai-weight-${delimiter ? "delim-" : ""}${direction}-${tier}`;
}

function weightTier(weight: number): number {
  if (weight > 1) {
    if (weight >= 4) return MAX_TIER;
    if (weight >= 2.5) return 3;
    if (weight >= 1.5) return 2;
    return 1;
  }
  const inverse = weight <= 0 ? Number.POSITIVE_INFINITY : 1 / weight;
  if (inverse >= 4) return MAX_TIER;
  if (inverse >= 2.5) return 3;
  if (inverse >= 1.5) return 2;
  return 1;
}

function markFor(classes: string[]): Decoration {
  const key = classes.join(" ");
  const cached = markCache.get(key);
  if (cached) return cached;
  const decoration = Decoration.mark({ class: key });
  markCache.set(key, decoration);
  return decoration;
}
