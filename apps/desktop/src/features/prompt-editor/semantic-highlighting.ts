import { RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import { promptAnalysisForState, type PromptSemanticSpan } from "./prompt-analysis";

const markCache = new Map<string, Decoration>();

export const naiSemanticHighlighting = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (!update.docChanged) return;
      this.decorations = buildDecorations(update.view);
    }
  },
  { decorations: (value) => value.decorations },
);

function buildDecorations(view: EditorView): DecorationSet {
  try {
    const builder = new RangeSetBuilder<Decoration>();
    for (const span of promptAnalysisForState(view.state).semanticSpans) {
      builder.add(span.from, span.to, markFor(classesForSpan(span)));
    }
    return builder.finish();
  } catch (error) {
    console.error("[NaiPromptEditor] semantic highlighting disabled after an error", error);
    return Decoration.none;
  }
}

function classesForSpan(span: PromptSemanticSpan): string[] {
  if (span.kind === "function") return ["nai-function"];
  if (span.kind === "weight_reset") return ["nai-semantic-reset"];
  return [
    "nai-semantic-weight",
    `nai-semantic-weight-${span.role}`,
    `nai-weight-${span.direction}-${span.tier}`,
  ];
}

function markFor(classes: string[]): Decoration {
  const key = classes.join(" ");
  const cached = markCache.get(key);
  if (cached) return cached;
  const decoration = Decoration.mark({ class: key });
  markCache.set(key, decoration);
  return decoration;
}
