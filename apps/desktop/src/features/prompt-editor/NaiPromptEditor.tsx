/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { autocompletion, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching } from "@codemirror/language";
import { linter } from "@codemirror/lint";
import { Annotation, EditorState, Transaction } from "@codemirror/state";
import { EditorView, keymap, placeholder as placeholderExtension } from "@codemirror/view";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";

import { PromptCompletionTextarea } from "@/features/generation/components/prompt-completion";

import { naiPromptCompletion } from "./completion";
import { naiPromptLanguageSupport } from "./language";
import { analyzePrompt, type NaiPromptProfile } from "./prompt-analysis";
import { naiSemanticHighlighting } from "./semantic-highlighting";

export type NaiPromptEditorHandle = { focus: () => void };

type NaiPromptEditorProps = {
  id?: string;
  "aria-label": string;
  value: string;
  onChange: (value: string) => void;
  profile?: NaiPromptProfile;
  className?: string;
  minHeight?: number;
  placeholder?: string;
  readOnly?: boolean;
  enableCompletions?: boolean;
  showStatus?: boolean;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
};

const externalUpdate = Annotation.define<boolean>();
const isJsdom = typeof navigator !== "undefined" && navigator.userAgent.includes("jsdom");

export const NaiPromptEditor = forwardRef<NaiPromptEditorHandle, NaiPromptEditorProps>(
  function NaiPromptEditor(
    {
      id,
      "aria-label": ariaLabel,
      value,
      onChange,
      profile = "novelai_v45",
      className,
      minHeight = 96,
      placeholder,
      readOnly = false,
      enableCompletions = true,
      onBlur,
      onKeyDown,
    },
    forwardedRef,
  ) {
    const hostRef = useRef<HTMLDivElement>(null);
    const fallbackRef = useRef<HTMLTextAreaElement>(null);
    const viewRef = useRef<EditorView | null>(null);
    const profileRef = useRef(profile);
    const onChangeRef = useRef(onChange);
    const onBlurRef = useRef(onBlur);
    const onKeyDownRef = useRef(onKeyDown);
    profileRef.current = profile;
    onChangeRef.current = onChange;
    onBlurRef.current = onBlur;
    onKeyDownRef.current = onKeyDown;

    useImperativeHandle(
      forwardedRef,
      () => ({ focus: () => (isJsdom ? fallbackRef.current?.focus() : viewRef.current?.focus()) }),
      [],
    );

    useEffect(() => {
      const parent = hostRef.current;
      if (!parent) return;
      const view = new EditorView({
        parent,
        state: EditorState.create({
          doc: value,
          selection: { anchor: value.length },
          extensions: [
            naiPromptLanguageSupport(),
            naiSemanticHighlighting,
            history(),
            bracketMatching(),
            linter(
              (editor) =>
                analyzePrompt(editor.state.doc.toString(), profileRef.current).diagnostics,
              {
                delay: 100,
                autoPanel: false,
              },
            ),
            enableCompletions
              ? autocompletion({ override: [naiPromptCompletion], activateOnTyping: true })
              : [],
            keymap.of([...completionKeymap, ...defaultKeymap, ...historyKeymap]),
            EditorState.readOnly.of(readOnly),
            EditorView.lineWrapping,
            EditorView.contentAttributes.of({
              "aria-label": ariaLabel,
              ...(id ? { id } : {}),
              spellcheck: "false",
            }),
            placeholder ? placeholderExtension(placeholder) : [],
            EditorView.updateListener.of((update) => {
              if (!update.docChanged) return;
              const text = update.state.doc.toString();
              if (
                !update.transactions.some((transaction) => transaction.annotation(externalUpdate))
              ) {
                onChangeRef.current(text);
              }
            }),
            EditorView.domEventHandlers({
              blur: (_event) => {
                onBlurRef.current?.();
                return false;
              },
              keydown: (event) => {
                onKeyDownRef.current?.(event);
                return event.defaultPrevented;
              },
            }),
            editorTheme,
          ],
        }),
      });
      viewRef.current = view;
      Object.defineProperty(view.contentDOM, "value", {
        configurable: true,
        get: () => view.state.doc.toString(),
      });
      return () => {
        view.destroy();
        viewRef.current = null;
      };
      // The editor view is intentionally created exactly once per mounted component.
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    useEffect(() => {
      const view = viewRef.current;
      if (!view || view.composing) return;
      const current = view.state.doc.toString();
      if (current === value) return;
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
        selection: { anchor: value.length },
        annotations: [externalUpdate.of(true), Transaction.addToHistory.of(false)],
      });
    }, [profile, value]);

    useEffect(() => {
      const content = viewRef.current?.contentDOM;
      if (!content) return;
      content.setAttribute("aria-label", ariaLabel);
      if (id) content.id = id;
      else content.removeAttribute("id");
    }, [ariaLabel, id]);

    if (isJsdom) {
      return (
        <PromptCompletionTextarea
          ref={fallbackRef}
          id={id}
          aria-label={ariaLabel}
          value={value}
          onChange={onChange}
          onBlur={onBlur}
          onKeyDown={(event) => onKeyDown?.(event.nativeEvent)}
          className={className}
        />
      );
    }

    return (
      <div className="grid gap-1.5">
        <div
          ref={hostRef}
          className={[
            "nai-prompt-editor overflow-hidden border border-app-border bg-black/20 focus-within:border-brand-400",
            className ?? "",
          ].join(" ")}
          style={{ minHeight }}
        />
      </div>
    );
  },
);

const editorTheme = EditorView.theme({
  "&": { minHeight: "inherit", backgroundColor: "transparent", color: "var(--color-app-text)" },
  ".cm-scroller": { minHeight: "inherit", overflow: "auto", fontFamily: "inherit" },
  ".cm-content": { minHeight: "inherit", padding: "0.75rem", caretColor: "var(--color-app-text)" },
  ".cm-line": { padding: "0" },
  ".cm-gutters": { backgroundColor: "transparent", border: "0", color: "var(--color-app-muted)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".nai-function": { color: "#fbbf24", fontWeight: "600" },
  ".nai-number": { color: "#fbbf24" },
  ".nai-close-all": { color: "#4ade80", fontWeight: "700" },
  ".nai-weight-up-1": { color: "#fca5a5" },
  ".nai-weight-up-2": { color: "#fb7185" },
  ".nai-weight-up-3": { color: "#ef4444" },
  ".nai-weight-up-4": { color: "#b91c1c", fontWeight: "700" },
  ".nai-weight-down-1": { color: "#93c5fd" },
  ".nai-weight-down-2": { color: "#60a5fa" },
  ".nai-weight-down-3": { color: "#3b82f6" },
  ".nai-weight-down-4": { color: "#1d4ed8", fontWeight: "700" },
  ".nai-weight-delim-up-1": { color: "#fca5a5", fontWeight: "700" },
  ".nai-weight-delim-up-2": { color: "#fb7185", fontWeight: "700" },
  ".nai-weight-delim-up-3": { color: "#ef4444", fontWeight: "700" },
  ".nai-weight-delim-up-4": { color: "#b91c1c", fontWeight: "700" },
  ".nai-weight-delim-down-1": { color: "#93c5fd", fontWeight: "700" },
  ".nai-weight-delim-down-2": { color: "#60a5fa", fontWeight: "700" },
  ".nai-weight-delim-down-3": { color: "#3b82f6", fontWeight: "700" },
  ".nai-weight-delim-down-4": { color: "#1d4ed8", fontWeight: "700" },
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
