/* eslint-disable react-perf/jsx-no-new-object-as-prop */
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef, useEffect, useImperativeHandle, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { generationApi } from "@/platform/atelier";
import type { ImageModelDto, PromptTokenCountDto } from "@/types";

import { createNaiPromptCompletion } from "./completion";
import type { NaiPromptHighlightMode } from "./editor-theme";
import type { NaiPromptProfile, PromptEditorMessages } from "./prompt-analysis";
import { usePromptEditorSettings } from "./prompt-editor-settings-context";
import { normalizeFullWidthPunctuation } from "./prompt-normalization";
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
  tokenCount?: PromptTokenCountDto | null;
  className?: string;
  minHeight?: number;
  placeholder?: string;
  readOnly?: boolean;
  enableCompletions?: boolean;
  highlightMode?: NaiPromptHighlightMode;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
  convertFullWidthPunctuation?: boolean;
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
      tokenCount,
      className,
      minHeight = 96,
      placeholder,
      readOnly = false,
      enableCompletions = true,
      highlightMode = "foreground",
      onBlur,
      onKeyDown,
      convertFullWidthPunctuation,
    },
    forwardedRef,
  ) {
    const inheritedConvertFullWidthPunctuation = usePromptEditorSettings();
    const shouldConvertFullWidthPunctuation =
      convertFullWidthPunctuation ?? inheritedConvertFullWidthPunctuation;
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
        convertFullWidthPunctuation: shouldConvertFullWidthPunctuation,
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
        {model ? (
          <PromptTokenMeter
            model={model}
            text={shouldConvertFullWidthPunctuation ? normalizeFullWidthPunctuation(value) : value}
            suppliedCount={tokenCount}
          />
        ) : null}
      </div>
    );
  },
);

function PromptTokenMeter({
  model,
  text,
  suppliedCount,
}: {
  model: ImageModelDto;
  text: string;
  suppliedCount?: PromptTokenCountDto | null;
}) {
  const { t } = useTranslation("promptEditor");
  const [remoteCount, setRemoteCount] = useState<PromptTokenCountDto | null>(null);

  useEffect(() => {
    if (suppliedCount !== undefined) return;
    let active = true;
    async function refreshCount() {
      try {
        const next = await generationApi.countPromptTokens({
          compile: {
            model,
            main_preset_id: null,
            prompt: text,
            negative_prompt: null,
            characters: [],
            max_depth: 16,
          },
          quality: "none",
          transparent_background: false,
          uc_preset: "none",
        });
        if (active) setRemoteCount(next.prompt);
      } catch {
        if (active) setRemoteCount(null);
      }
    }
    const timer = window.setTimeout(() => {
      void refreshCount();
    }, 160);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [model, suppliedCount, text]);

  const count = suppliedCount === undefined ? remoteCount : suppliedCount;

  if (!count) return <div aria-hidden="true" className="h-3" />;

  const label = t("tokenUsage", count);
  const overLimit = count.used > count.limit;
  return (
    <div className="flex h-3 items-center gap-1.5">
      <progress
        aria-label={label}
        max={count.limit}
        value={Math.min(count.used, count.limit)}
        className={[
          "h-px min-w-0 flex-1 appearance-none overflow-hidden bg-app-border [&::-webkit-progress-bar]:bg-app-border",
          overLimit
            ? "[&::-moz-progress-bar]:bg-red-400 [&::-webkit-progress-value]:bg-red-400"
            : "[&::-moz-progress-bar]:bg-brand-400 [&::-webkit-progress-value]:bg-brand-400",
        ].join(" ")}
      />
      <span
        title={label}
        className={[
          "min-w-8 text-right font-mono text-[9px] leading-none",
          overLimit ? "text-red-300" : "text-app-muted",
        ].join(" ")}
      >
        {count.used}
      </span>
    </div>
  );
}

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
