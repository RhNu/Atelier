import type { PromptToken } from "./prompt-tokenizer";

export type PromptWeightDirection = "up" | "down" | "neutral";
export type PromptSemanticSpan =
  | { kind: "function"; from: number; to: number }
  | { kind: "weight_reset"; from: number; to: number }
  | {
      kind: "weight";
      role: "content" | "operator";
      direction: PromptWeightDirection;
      tier: number;
      from: number;
      to: number;
    };

const UP_FACTOR = 1.05;
const DOWN_FACTOR = 1 / UP_FACTOR;
const MAX_WEIGHT_TIER = 4;
const WEIGHT_EPSILON = 0.0001;

export function buildPromptSemanticSpans(tokens: PromptToken[]): PromptSemanticSpan[] {
  const spans: PromptSemanticSpan[] = [];
  const functionRanges = findFunctionRanges(tokens);
  const functionTokens = new Set<number>();

  for (const range of functionRanges) {
    for (let index = range.fromIndex; index <= range.toIndex; index += 1) {
      functionTokens.add(index);
    }
    spans.push({
      kind: "function",
      from: tokens[range.fromIndex]?.from ?? 0,
      to: tokens[range.toIndex]?.to ?? tokens[range.fromIndex]?.to ?? 0,
    });
  }

  const weightStack: Array<{ kind: "up" | "down" | "numeric"; factor: number }> = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token || functionTokens.has(index)) continue;

    const numericColon = tokens[index + 1];
    if (token.kind === "number" && numericColon?.kind === "double_colon") {
      const factor = Number(token.text);
      spans.push(weightSpan(token.from, numericColon.to, "operator", factor));
      if (Number.isFinite(factor)) weightStack.push({ kind: "numeric", factor });
      index += 1;
      continue;
    }

    if (token.kind === "double_colon") {
      spans.push({ kind: "weight_reset", from: token.from, to: token.to });
      weightStack.length = 0;
      continue;
    }

    if (token.text === "{") {
      spans.push(weightSpan(token.from, token.to, "operator", UP_FACTOR));
      weightStack.push({ kind: "up", factor: UP_FACTOR });
      continue;
    }
    if (token.text === "[") {
      spans.push(weightSpan(token.from, token.to, "operator", DOWN_FACTOR));
      weightStack.push({ kind: "down", factor: DOWN_FACTOR });
      continue;
    }
    if (token.text === "}" || token.text === "]") {
      const direction = token.text === "}" ? "up" : "down";
      const factor = direction === "up" ? UP_FACTOR : DOWN_FACTOR;
      spans.push(weightSpan(token.from, token.to, "operator", factor));
      popWeight(weightStack, direction);
      continue;
    }

    if (!isWeightContent(token)) continue;
    const effectiveWeight = weightStack.reduce((weight, entry) => weight * entry.factor, 1);
    if (Math.abs(effectiveWeight - 1) <= WEIGHT_EPSILON) continue;
    spans.push(weightSpan(token.from, token.to, "content", effectiveWeight));
  }

  return spans.sort((left, right) => left.from - right.from || left.to - right.to);
}

function findFunctionRanges(tokens: PromptToken[]): Array<{ fromIndex: number; toIndex: number }> {
  const ranges: Array<{ fromIndex: number; toIndex: number }> = [];
  for (let index = 0; index + 2 < tokens.length; index += 1) {
    if (
      tokens[index]?.text !== "$" ||
      tokens[index + 1]?.kind !== "identifier" ||
      tokens[index + 2]?.text !== "("
    ) {
      continue;
    }
    let depth = 1;
    let closeIndex = tokens.length - 1;
    for (let cursor = index + 3; cursor < tokens.length; cursor += 1) {
      const token = tokens[cursor];
      if (token?.text === "(") depth += 1;
      if (token?.text === ")") depth -= 1;
      if (depth === 0) {
        closeIndex = cursor;
        break;
      }
    }
    ranges.push({ fromIndex: index, toIndex: closeIndex });
    index = closeIndex;
  }
  return ranges;
}

function isWeightContent(token: PromptToken): boolean {
  return (
    token.kind === "number" ||
    token.kind === "invalid_number" ||
    token.kind === "tag" ||
    token.kind === "identifier" ||
    token.kind === "string" ||
    token.kind === "unterminated_string" ||
    token.kind === "escaped" ||
    token.kind === "text"
  );
}

function weightSpan(
  from: number,
  to: number,
  role: "content" | "operator",
  weight: number,
): PromptSemanticSpan {
  return {
    kind: "weight",
    role,
    direction: weightDirection(weight),
    tier: weightTier(weight),
    from,
    to,
  };
}

function weightDirection(weight: number): PromptWeightDirection {
  if (weight > 1 + WEIGHT_EPSILON) return "up";
  if (weight < 1 - WEIGHT_EPSILON) return "down";
  return "neutral";
}

function weightTier(weight: number): number {
  if (weight > 1) {
    if (weight >= 4) return MAX_WEIGHT_TIER;
    if (weight >= 2.5) return 3;
    if (weight >= 1.5) return 2;
    return 1;
  }
  const inverse = weight <= 0 ? Number.POSITIVE_INFINITY : 1 / weight;
  if (inverse >= 4) return MAX_WEIGHT_TIER;
  if (inverse >= 2.5) return 3;
  if (inverse >= 1.5) return 2;
  return 1;
}

function popWeight(
  stack: Array<{ kind: "up" | "down" | "numeric"; factor: number }>,
  kind: "up" | "down",
) {
  const index = stack.findLastIndex((entry) => entry.kind === kind);
  if (index >= 0) stack.splice(index, 1);
}
