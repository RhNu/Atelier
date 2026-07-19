import {
  acceptCompletion,
  autocompletion,
  completionKeymap,
  type CompletionSource,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching } from "@codemirror/language";
import { forceLinting, linter } from "@codemirror/lint";
import { Annotation, Compartment, EditorState, Transaction } from "@codemirror/state";
import { EditorView, keymap, placeholder as placeholderExtension } from "@codemirror/view";
import type { RefObject } from "react";

import {
  applyHighlightMode,
  naiPromptEditorTheme,
  type NaiPromptHighlightMode,
} from "./editor-theme";
import { naiPromptLanguageSupport } from "./language";
import {
  naiPromptMessagesFacet,
  naiPromptProfileFacet,
  promptAnalysisForState,
  type NaiPromptProfile,
  type PromptEditorMessages,
} from "./prompt-analysis";
import { naiSemanticHighlighting } from "./semantic-highlighting";

export type PromptEditorCompartments = ReturnType<typeof createPromptEditorCompartments>;
export type PromptEditorRuntimeRefs = {
  view: RefObject<EditorView | null>;
  isComposing: RefObject<boolean>;
  pendingExternalValue: RefObject<string | null>;
  onChange: RefObject<(value: string) => void>;
  onBlur: RefObject<(() => void) | undefined>;
  onKeyDown: RefObject<((event: KeyboardEvent) => void) | undefined>;
};
export type PromptEditorConfiguration = {
  id?: string;
  ariaLabel: string;
  value: string;
  profile: NaiPromptProfile;
  placeholder?: string;
  readOnly: boolean;
  enableCompletions: boolean;
  highlightMode: NaiPromptHighlightMode;
  messages: PromptEditorMessages;
  completionsPhrase: string;
  completionSource: CompletionSource;
};

const externalUpdate = Annotation.define<boolean>();

export function createPromptEditorCompartments() {
  return {
    attributes: new Compartment(),
    completion: new Compartment(),
    messages: new Compartment(),
    placeholder: new Compartment(),
    profile: new Compartment(),
    readOnly: new Compartment(),
  };
}

export function createPromptEditorView(
  parent: HTMLElement,
  configuration: PromptEditorConfiguration,
  compartments: PromptEditorCompartments,
  runtime: PromptEditorRuntimeRefs,
): EditorView {
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc: configuration.value,
      selection: { anchor: configuration.value.length },
      extensions: editorExtensions(configuration, compartments, runtime),
    }),
  });
  applyHighlightMode(view, configuration.highlightMode);
  return view;
}

function editorExtensions(
  configuration: PromptEditorConfiguration,
  compartments: PromptEditorCompartments,
  runtime: PromptEditorRuntimeRefs,
) {
  return [
    naiPromptLanguageSupport(),
    naiSemanticHighlighting,
    history(),
    bracketMatching(),
    linter((editor) => promptAnalysisForState(editor.state).diagnostics, {
      delay: 100,
      autoPanel: false,
    }),
    compartments.completion.of(
      completionExtension(configuration.enableCompletions, configuration.completionSource),
    ),
    keymap.of([
      { key: "Tab", run: acceptCompletion },
      ...completionKeymap,
      ...defaultKeymap,
      ...historyKeymap,
    ]),
    compartments.readOnly.of(readOnlyExtensions(configuration.readOnly)),
    compartments.profile.of(naiPromptProfileFacet.of(configuration.profile)),
    compartments.messages.of(
      messageExtensions(configuration.messages, configuration.completionsPhrase),
    ),
    EditorView.lineWrapping,
    compartments.attributes.of(contentAttributes(configuration.id, configuration.ariaLabel)),
    compartments.placeholder.of(
      configuration.placeholder ? placeholderExtension(configuration.placeholder) : [],
    ),
    EditorView.updateListener.of((update) => {
      if (
        update.docChanged &&
        !update.transactions.some((transaction) => transaction.annotation(externalUpdate))
      ) {
        runtime.onChange.current(update.state.doc.toString());
      }
    }),
    EditorView.domEventHandlers({
      blur: () => {
        runtime.onBlur.current?.();
        return false;
      },
      keydown: (event) => {
        runtime.onKeyDown.current?.(event);
        return event.defaultPrevented;
      },
      compositionstart: () => {
        runtime.isComposing.current = true;
        return false;
      },
      compositionend: () => {
        runtime.isComposing.current = false;
        return handleCompositionEnd(runtime);
      },
    }),
    naiPromptEditorTheme,
  ];
}

export function applyControlledValue(view: EditorView, value: string) {
  const current = view.state.doc.toString();
  if (current === value) return;
  const anchor = Math.min(view.state.selection.main.head, value.length);
  view.dispatch({
    changes: { from: 0, to: current.length, insert: value },
    selection: { anchor },
    annotations: [externalUpdate.of(true), Transaction.addToHistory.of(false)],
  });
}

export function reconfigurePromptProfile(
  view: EditorView,
  compartment: Compartment,
  profile: NaiPromptProfile,
) {
  view.dispatch({ effects: compartment.reconfigure(naiPromptProfileFacet.of(profile)) });
  forceLinting(view);
}

export function reconfigurePromptMessages(
  view: EditorView,
  compartment: Compartment,
  messages: PromptEditorMessages,
  completionsPhrase: string,
) {
  view.dispatch({
    effects: compartment.reconfigure(messageExtensions(messages, completionsPhrase)),
  });
  forceLinting(view);
}

export function completionExtension(enabled: boolean, source: CompletionSource) {
  return enabled
    ? autocompletion({
        override: [source],
        activateOnTyping: true,
        activateOnTypingDelay: 120,
        interactionDelay: 0,
      })
    : [];
}

export function contentAttributes(id: string | undefined, ariaLabel: string) {
  return EditorView.contentAttributes.of({
    "aria-label": ariaLabel,
    ...(id ? { id } : {}),
    spellcheck: "false",
  });
}

export function promptPlaceholder(value: string | undefined) {
  return value ? placeholderExtension(value) : [];
}

export function readOnlyExtensions(readOnly: boolean) {
  return [EditorState.readOnly.of(readOnly), EditorView.editable.of(!readOnly)];
}

function messageExtensions(messages: PromptEditorMessages, completionsPhrase: string) {
  return [
    naiPromptMessagesFacet.of(messages),
    EditorState.phrases.of({ Completions: completionsPhrase }),
  ];
}

function handleCompositionEnd(runtime: PromptEditorRuntimeRefs): false {
  queueMicrotask(() => {
    const view = runtime.view.current;
    const pending = runtime.pendingExternalValue.current;
    if (view && pending !== null && !view.composing) {
      applyControlledValue(view, pending);
      runtime.pendingExternalValue.current = null;
    }
  });
  return false;
}
