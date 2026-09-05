import { EditorView } from "@codemirror/view";

export type NaiPromptHighlightMode = "foreground" | "background";

export const naiPromptEditorTheme = EditorView.theme({
  "&": {
    minHeight: "inherit",
    backgroundColor: "transparent",
    color: "var(--color-app-text)",
    fontFamily: "var(--font-prompt)",
    "--nai-function-foreground": "#67e8f9",
    "--nai-comment-foreground": "rgb(103 232 249 / 0.58)",
  },
  ".cm-scroller": {
    minHeight: "inherit",
    overflow: "auto",
    fontFamily: "var(--font-prompt)",
  },
  ".cm-content": { minHeight: "inherit", padding: "0.75rem", caretColor: "var(--color-app-text)" },
  ".cm-line": { padding: "0" },
  ".cm-gutters": { backgroundColor: "transparent", border: "0", color: "var(--color-app-muted)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".nai-function": {
    color: "var(--nai-function-foreground) !important",
    fontWeight: "600",
  },
  ".nai-function *": { color: "inherit !important" },
  ".nai-function-comment": {
    "--nai-function-foreground": "var(--nai-comment-foreground)",
    fontWeight: "400",
  },
  ".nai-number": { color: "#fbbf24" },
  ".nai-semantic-weight": { color: "var(--nai-weight-foreground) !important" },
  ".nai-semantic-weight *": { color: "inherit !important" },
  ".nai-semantic-weight-operator": { fontWeight: "700" },
  ".nai-semantic-reset": { color: "#4ade80 !important", fontWeight: "700" },
  ".nai-semantic-reset *": { color: "inherit !important" },
  ".nai-weight-up-1": {
    "--nai-weight-foreground": "#fca5a5",
    "--nai-weight-background": "rgb(252 165 165 / 0.24)",
  },
  ".nai-weight-up-2": {
    "--nai-weight-foreground": "#fb7185",
    "--nai-weight-background": "rgb(251 113 133 / 0.28)",
  },
  ".nai-weight-up-3": {
    "--nai-weight-foreground": "#ef4444",
    "--nai-weight-background": "rgb(239 68 68 / 0.32)",
  },
  ".nai-weight-up-4": {
    "--nai-weight-foreground": "#b91c1c",
    "--nai-weight-background": "rgb(185 28 28 / 0.38)",
    fontWeight: "700",
  },
  ".nai-weight-down-1": {
    "--nai-weight-foreground": "#93c5fd",
    "--nai-weight-background": "rgb(147 197 253 / 0.24)",
  },
  ".nai-weight-down-2": {
    "--nai-weight-foreground": "#60a5fa",
    "--nai-weight-background": "rgb(96 165 250 / 0.28)",
  },
  ".nai-weight-down-3": {
    "--nai-weight-foreground": "#3b82f6",
    "--nai-weight-background": "rgb(59 130 246 / 0.32)",
  },
  ".nai-weight-down-4": {
    "--nai-weight-foreground": "#1d4ed8",
    "--nai-weight-background": "rgb(29 78 216 / 0.38)",
    fontWeight: "700",
  },
  ".nai-weight-neutral-1": {
    "--nai-weight-foreground": "#fbbf24",
    "--nai-weight-background": "rgb(251 191 36 / 0.26)",
  },
  "&.nai-highlight-background .nai-semantic-weight": {
    color: "var(--color-app-text) !important",
    backgroundColor: "var(--nai-weight-background)",
    borderRadius: "0",
    boxDecorationBreak: "clone",
    WebkitBoxDecorationBreak: "clone",
  },
  "&.nai-highlight-background .nai-semantic-reset": {
    color: "var(--color-app-text) !important",
    backgroundColor: "rgb(74 222 128 / 0.3)",
    borderRadius: "0",
    boxDecorationBreak: "clone",
    WebkitBoxDecorationBreak: "clone",
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "rgb(59 130 246 / 0.28)",
  },
  ".cm-tooltip": {
    border: "1px solid var(--color-app-border)",
    borderRadius: "0",
    backgroundColor: "var(--color-app-panel)",
    color: "var(--color-app-text)",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "rgb(59 130 246 / 0.2)",
    color: "var(--color-app-text)",
  },
  "&.cm-focused": { outline: "none" },
});

export function applyHighlightMode(view: EditorView, mode: NaiPromptHighlightMode) {
  view.dom.classList.toggle("nai-highlight-foreground", mode === "foreground");
  view.dom.classList.toggle("nai-highlight-background", mode === "background");
}
