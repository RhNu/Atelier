import { EditorView } from "@codemirror/view";
import { useEffect, useRef } from "react";

import {
  applyControlledValue,
  completionExtension,
  contentAttributes,
  createPromptEditorCompartments,
  createPromptEditorView,
  fullWidthPunctuationExtension,
  normalizeFullWidthPunctuationInView,
  promptPlaceholder,
  readOnlyExtensions,
  reconfigurePromptMessages,
  reconfigurePromptProfile,
  type PromptEditorConfiguration,
  type PromptEditorRuntimeRefs,
} from "./editor-adapter";
import { applyHighlightMode } from "./editor-theme";

type PromptEditorCallbacks = {
  onChange: (value: string) => void;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
};

export function usePromptEditor(
  configuration: PromptEditorConfiguration,
  callbacks: PromptEditorCallbacks,
) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const isComposingRef = useRef(false);
  const convertFullWidthPunctuationRef = useRef(configuration.convertFullWidthPunctuation);
  const pendingExternalValueRef = useRef<string | null>(null);
  const onChangeRef = useRef(callbacks.onChange);
  const onBlurRef = useRef(callbacks.onBlur);
  const onKeyDownRef = useRef(callbacks.onKeyDown);
  const compartmentsRef = useRef<ReturnType<typeof createPromptEditorCompartments> | null>(null);
  compartmentsRef.current ??= createPromptEditorCompartments();
  const compartments = compartmentsRef.current;
  const initialConfigurationRef = useRef(configuration);
  onChangeRef.current = callbacks.onChange;
  onBlurRef.current = callbacks.onBlur;
  onKeyDownRef.current = callbacks.onKeyDown;
  convertFullWidthPunctuationRef.current = configuration.convertFullWidthPunctuation;
  const runtimeRef = useRef<PromptEditorRuntimeRefs | null>(null);
  runtimeRef.current ??= {
    view: viewRef,
    isComposing: isComposingRef,
    convertFullWidthPunctuation: convertFullWidthPunctuationRef,
    pendingExternalValue: pendingExternalValueRef,
    onChange: onChangeRef,
    onBlur: onBlurRef,
    onKeyDown: onKeyDownRef,
  };
  const runtime = runtimeRef.current;

  useEffect(() => {
    const parent = hostRef.current;
    if (!parent) return;
    const view = createPromptEditorView(
      parent,
      initialConfigurationRef.current,
      compartments,
      runtime,
    );
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [compartments, runtime]);

  useControlledValue(viewRef, isComposingRef, pendingExternalValueRef, configuration.value);
  usePromptEditorConfiguration(viewRef, isComposingRef, compartments, configuration);
  return { hostRef, viewRef };
}

function useControlledValue(
  viewRef: React.RefObject<EditorView | null>,
  isComposingRef: React.RefObject<boolean>,
  pendingExternalValueRef: React.RefObject<string | null>,
  value: string,
) {
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (isComposingRef.current || view.composing) {
      pendingExternalValueRef.current = value;
      return;
    }
    applyControlledValue(view, value);
    pendingExternalValueRef.current = null;
  }, [isComposingRef, pendingExternalValueRef, value, viewRef]);
}

function usePromptEditorConfiguration(
  viewRef: React.RefObject<EditorView | null>,
  isComposingRef: React.RefObject<boolean>,
  compartments: ReturnType<typeof createPromptEditorCompartments>,
  configuration: PromptEditorConfiguration,
) {
  useEffect(() => {
    const view = viewRef.current;
    if (view) {
      reconfigurePromptProfile(view, compartments.profile, configuration.profile);
    }
  }, [compartments.profile, configuration.profile, viewRef]);
  useEffect(() => {
    const view = viewRef.current;
    if (view) {
      reconfigurePromptMessages(
        view,
        compartments.messages,
        configuration.messages,
        configuration.completionsPhrase,
      );
    }
  }, [compartments.messages, configuration.completionsPhrase, configuration.messages, viewRef]);
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.completion.reconfigure(
        completionExtension(configuration.enableCompletions, configuration.completionSource),
      ),
    });
  }, [
    compartments.completion,
    configuration.completionSource,
    configuration.enableCompletions,
    viewRef,
  ]);
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.readOnly.reconfigure(readOnlyExtensions(configuration.readOnly)),
    });
  }, [compartments.readOnly, configuration.readOnly, viewRef]);
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.placeholder.reconfigure(promptPlaceholder(configuration.placeholder)),
    });
  }, [compartments.placeholder, configuration.placeholder, viewRef]);
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: compartments.fullWidthPunctuation.reconfigure(
        fullWidthPunctuationExtension(configuration.convertFullWidthPunctuation, isComposingRef),
      ),
    });
    if (configuration.convertFullWidthPunctuation) {
      normalizeFullWidthPunctuationInView(view);
    }
  }, [
    compartments.fullWidthPunctuation,
    configuration.convertFullWidthPunctuation,
    isComposingRef,
    viewRef,
  ]);
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.attributes.reconfigure(
        contentAttributes(configuration.id, configuration.ariaLabel),
      ),
    });
  }, [compartments.attributes, configuration.ariaLabel, configuration.id, viewRef]);
  useEffect(() => {
    const view = viewRef.current;
    if (view) applyHighlightMode(view, configuration.highlightMode);
  }, [configuration.highlightMode, viewRef]);
}
