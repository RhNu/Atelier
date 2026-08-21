/* eslint-disable react-perf/jsx-no-new-object-as-prop */
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef, useImperativeHandle, useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { ImageModelDto } from "@/types";

import { createNaiPromptCompletion } from "./completion";
import type { NaiPromptHighlightMode } from "./editor-theme";
import type { NaiPromptProfile, PromptEditorMessages } from "./prompt-analysis";
import { usePromptEditor } from "./use-prompt-editor";

export type NaiPromptEditorHandle = { focus: () => void };
export type { NaiPromptHighlightMode } from "./editor-theme";

type NaiPromptEditorProps = {
  id?: string;
  "aria-label": string;
  value: string;
  onChange: (value: string) => void;
  profile?: NaiPromptProfile;
  model?: ImageModelDto;
  className?: string;
  minHeight?: number;
  placeholder?: string;
  readOnly?: boolean;
  enableCompletions?: boolean;
  highlightMode?: NaiPromptHighlightMode;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
};

export const NaiPromptEditor = forwardRef<NaiPromptEditorHandle, NaiPromptEditorProps>(
  function NaiPromptEditor(
    {
      id,
      "aria-label": ariaLabel,
      value,
      onChange,
      profile = "novelai_v45",
      model,
      className,
      minHeight = 96,
      placeholder,
      readOnly = false,
      enableCompletions = true,
      highlightMode = "foreground",
      onBlur,
      onKeyDown,
    },
    forwardedRef,
  ) {
    const { t } = useTranslation("promptEditor");
    const queryClient = useQueryClient();
    const messages = usePromptEditorMessages(t);
    const completionSource = useMemo(
      () =>
        createNaiPromptCompletion(
          queryClient,
          {
            reusableChunk: t("reusableChunk"),
            compileTimeComment: t("compileTimeComment"),
            promptChunk: t("promptChunk"),
          },
          model ?? null,
        ),
      [model, queryClient, t],
    );
    const { hostRef, viewRef } = usePromptEditor(
      {
        id,
        ariaLabel,
        value,
        profile,
        placeholder,
        readOnly,
        enableCompletions,
        highlightMode,
        messages,
        completionsPhrase: t("completions"),
        completionSource,
      },
      { onChange, onBlur, onKeyDown },
    );
    useImperativeHandle(forwardedRef, () => ({ focus: () => viewRef.current?.focus() }), [viewRef]);

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

type PromptEditorTranslator = ReturnType<typeof useTranslation<"promptEditor">>["t"];

function usePromptEditorMessages(t: PromptEditorTranslator): PromptEditorMessages {
  return useMemo(
    () => ({
      unmatchedStrengtheningClose: t("diagnostic.unmatchedStrengtheningClose"),
      unmatchedWeakeningClose: t("diagnostic.unmatchedWeakeningClose"),
      unclosedStrengthening: t("diagnostic.unclosedStrengthening"),
      unclosedWeakening: t("diagnostic.unclosedWeakening"),
      invalidNumericWeight: t("diagnostic.invalidNumericWeight"),
      unsupportedNumericWeight: t("diagnostic.unsupportedNumericWeight"),
      unsupportedNegativeNumericWeight: t("diagnostic.unsupportedNegativeNumericWeight"),
      unclosedNumericWeight: t("diagnostic.unclosedNumericWeight"),
      unterminatedString: t("diagnostic.unterminatedString"),
      unclosedRandomizer: t("diagnostic.unclosedRandomizer"),
      emptyRandomizerOption: t("diagnostic.emptyRandomizerOption"),
      unclosedFunctionCall: t("diagnostic.unclosedFunctionCall"),
      unknownFunction: t("diagnostic.unknownFunction"),
      invalidFunctionArity: t("diagnostic.invalidFunctionArity"),
      invalidFunctionArgument: t("diagnostic.invalidFunctionArgument"),
    }),
    [t],
  );
}
