import { acceptCompletion, closeCompletion, startCompletion } from "@codemirror/autocomplete";
import { undo } from "@codemirror/commands";
import { Transaction } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { act } from "@testing-library/react";

export function promptEditorText(element: HTMLElement): string {
  return promptEditorView(element).state.doc.toString();
}

export function typeInPromptEditor(element: HTMLElement, text: string) {
  const view = promptEditorView(element);
  const selection = view.state.selection.main;
  act(() => {
    view.focus();
    view.dispatch({
      changes: { from: selection.from, to: selection.to, insert: text },
      selection: { anchor: selection.from + text.length },
      annotations: Transaction.userEvent.of("input.type"),
    });
  });
}

export function clearPromptEditor(element: HTMLElement) {
  const view = promptEditorView(element);
  act(() => {
    view.focus();
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: "" },
      selection: { anchor: 0 },
      annotations: Transaction.userEvent.of("input.type"),
    });
  });
}

export function closePromptCompletion(element: HTMLElement): boolean {
  return runEditorCommand(element, closeCompletion);
}

export function acceptPromptCompletion(element: HTMLElement): boolean {
  return runEditorCommand(element, acceptCompletion);
}

export function startPromptCompletion(element: HTMLElement): boolean {
  return runEditorCommand(element, startCompletion);
}

export function undoPromptEditor(element: HTMLElement): boolean {
  return runEditorCommand(element, undo);
}

export function promptEditorView(element: HTMLElement): EditorView {
  const view = EditorView.findFromDOM(element);
  if (!view) throw new Error("Expected an element owned by a CodeMirror EditorView.");
  return view;
}

function runEditorCommand(element: HTMLElement, command: (view: EditorView) => boolean): boolean {
  let handled = false;
  act(() => {
    handled = command(promptEditorView(element));
  });
  return handled;
}
