import {
  HighlightStyle,
  LRLanguage,
  LanguageSupport,
  syntaxHighlighting,
} from "@codemirror/language";
import { styleTags, tags } from "@lezer/highlight";

import { parser } from "./nai-prompt-parser";

const naiPromptParser = parser.configure({
  props: [
    styleTags({
      String: tags.string,
      "Dollar Equals DoublePipe Pipe": tags.operator,
      Comma: tags.separator,
      "⚠": tags.invalid,
    }),
  ],
});

export const naiPromptLanguage = LRLanguage.define({
  name: "nai-prompt",
  parser: naiPromptParser,
});

const promptHighlightStyle = HighlightStyle.define([
  { tag: tags.number, color: "#fbbf24" },
  { tag: tags.string, color: "#86efac" },
  { tag: tags.function(tags.variableName), color: "#67e8f9", fontWeight: "600" },
  { tag: tags.operator, color: "#f0abfc" },
  { tag: tags.separator, color: "var(--color-app-muted)" },
  { tag: tags.invalid, color: "#fda4af", textDecoration: "underline wavy" },
]);

export function naiPromptLanguageSupport(): LanguageSupport {
  return new LanguageSupport(naiPromptLanguage, [syntaxHighlighting(promptHighlightStyle)]);
}
