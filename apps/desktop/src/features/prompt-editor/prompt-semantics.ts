import type { PromptSemanticSpan, PromptWeightDirection } from "./prompt-analysis";
import {
  firstPromptDescendant,
  hasPromptAncestor,
  promptDescendants,
  type PromptSyntax,
  syntaxRecordKey,
} from "./prompt-syntax-tree";

const UP_FACTOR = 1.05;
const DOWN_FACTOR = 1 / UP_FACTOR;
const MAX_WEIGHT_TIER = 4;
const WEIGHT_EPSILON = 0.0001;

type WeightLayer = { factor: number };
type StructuralLayer = { kind: "up" | "down"; weight?: WeightLayer };

export function buildPromptSemanticSpans(text: string, syntax: PromptSyntax): PromptSemanticSpan[] {
  const spans: PromptSemanticSpan[] = syntax.nodes
    .filter((node) => node.name === "ExtensionCall")
    .flatMap((node) => functionSemanticSpan(text, syntax, node));
  const numericOpen = numericOpenOperators(text, syntax);
  const openingColonKeys = new Set(numericOpen.map((item) => syntaxRecordKey(item.colon)));
  const openingNumberKeys = new Map(
    numericOpen.map((item) => [syntaxRecordKey(item.number), item]),
  );
  const weights: WeightLayer[] = [];
  const structures: StructuralLayer[] = [];

  for (const leaf of syntax.leaves) {
    if (leaf.from === leaf.to || hasPromptAncestor(leaf, "ExtensionCall")) continue;
    const numeric = openingNumberKeys.get(syntaxRecordKey(leaf));
    if (numeric) {
      spans.push(weightSpan(numeric.number.from, numeric.colon.to, "operator", numeric.factor));
      if (Number.isFinite(numeric.factor)) weights.push({ factor: numeric.factor });
      continue;
    }
    if (openingColonKeys.has(syntaxRecordKey(leaf))) continue;
    if (leaf.name === "DoubleColon") {
      spans.push({ kind: "weight_reset", from: leaf.from, to: leaf.to });
      weights.length = 0;
      for (const structure of structures) structure.weight = undefined;
      continue;
    }
    if (leaf.name === "LBrace" || leaf.name === "LBracket") {
      const up = leaf.name === "LBrace";
      const factor = up ? UP_FACTOR : DOWN_FACTOR;
      const weight = { factor };
      spans.push(weightSpan(leaf.from, leaf.to, "operator", factor));
      weights.push(weight);
      structures.push({ kind: up ? "up" : "down", weight });
      continue;
    }
    if (leaf.name === "RBrace" || leaf.name === "RBracket") {
      const up = leaf.name === "RBrace";
      spans.push(weightSpan(leaf.from, leaf.to, "operator", up ? UP_FACTOR : DOWN_FACTOR));
      closeStructuralLayer(structures, weights, up ? "up" : "down");
      continue;
    }
    if (!isWeightContent(leaf.name)) continue;
    const effectiveWeight = weights.reduce((weight, entry) => weight * entry.factor, 1);
    if (Math.abs(effectiveWeight - 1) > WEIGHT_EPSILON) {
      spans.push(weightSpan(leaf.from, leaf.to, "content", effectiveWeight));
    }
  }
  return spans.sort((left, right) => left.from - right.from || left.to - right.to);
}

function functionSemanticSpan(
  text: string,
  syntax: PromptSyntax,
  node: PromptSyntax["nodes"][number],
): PromptSemanticSpan[] {
  const identifier = firstPromptDescendant(syntax, node, "Identifier");
  const dollar = firstPromptDescendant(syntax, node, "Dollar");
  if (!identifier || !dollar) return [];
  const appearance =
    text.slice(identifier.from, identifier.to) === "comment" ? "comment" : "default";
  return [
    {
      kind: "function",
      appearance,
      from: dollar.from,
      to: appearance === "comment" ? node.to : identifier.to,
    },
  ];
}

function numericOpenOperators(text: string, syntax: PromptSyntax) {
  return syntax.nodes
    .filter((node) => node.name === "NumericEmphasis")
    .flatMap((node) => {
      const number =
        firstPromptDescendant(syntax, node, "Number") ??
        firstPromptDescendant(syntax, node, "InvalidNumber");
      const colon = promptDescendants(syntax, node, "DoubleColon")[0];
      return number
        ? colon
          ? [{ number, colon, factor: Number(text.slice(number.from, number.to)) }]
          : []
        : [];
    });
}

function closeStructuralLayer(
  structures: StructuralLayer[],
  weights: WeightLayer[],
  kind: StructuralLayer["kind"],
) {
  const index = structures.findLastIndex((entry) => entry.kind === kind);
  if (index < 0) return;
  const [structure] = structures.splice(index, 1);
  if (!structure?.weight) return;
  const weightIndex = weights.lastIndexOf(structure.weight);
  if (weightIndex >= 0) weights.splice(weightIndex, 1);
}

function isWeightContent(name: string): boolean {
  return [
    "Number",
    "InvalidNumber",
    "Tag",
    "Identifier",
    "String",
    "UnterminatedString",
    "Escaped",
    "Text",
  ].includes(name);
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
