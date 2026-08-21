import type { TFunction } from "i18next";
/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Settings2 } from "lucide-react";
import { forwardRef, useCallback, useImperativeHandle, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppSelect, AppTabs } from "@/components/ui";
import {
  NaiPromptEditor,
  promptProfileForModel,
  type NaiPromptEditorHandle,
} from "@/features/prompt-editor";
import type {
  ImageModelDescriptorDto,
  ImageModelDto,
  ModelCapabilitiesDto,
  PromptPresetDto,
} from "@/types";

import type { GenerationDraft } from "../model/generation-draft";
import {
  generationModelSelectOptions,
  generationUcPresetOptions,
  toImageModel,
  toQualityPreset,
  toUcPreset,
  toSelectOptions,
} from "../model/generation-options";
import { applyPromptPreset } from "../model/prompt-preset-model";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { GenerationPresetControl } from "./GenerationPresetControl";

type PromptTab = "positive" | "negative";

export type GenerationPromptPanelHandle = {
  focusPositive: () => void;
};

type GenerationPromptPanelProps = {
  draft: GenerationDraft;
  mainPresets: ReadonlyArray<PromptPresetDto>;
  mainPresetsPending: boolean;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  onFlush: () => void;
  onModelChange?: (model: ImageModelDto) => void;
  modelCatalog?: ReadonlyArray<ImageModelDescriptorDto>;
  capabilities?: ModelCapabilitiesDto;
};

function localizedUcPresetOptions(translate: TFunction<"generation">) {
  return toSelectOptions(generationUcPresetOptions, {
    heavy: translate("ucPresetOptions.heavy"),
    light: translate("ucPresetOptions.light"),
    furry_focus: translate("ucPresetOptions.furry_focus"),
    human_focus: translate("ucPresetOptions.human_focus"),
    none: translate("ucPresetOptions.none"),
  });
}

function PromptOptionsMenu({
  draft,
  capabilities,
  onPatch,
  onFlush,
}: Pick<GenerationPromptPanelProps, "draft" | "capabilities" | "onPatch" | "onFlush">) {
  const { t } = useTranslation("generation");
  const ucPresetOptions = useMemo(() => localizedUcPresetOptions(t), [t]);
  const qualityOptions = useMemo(
    () => [
      { value: "standard", label: "Standard" },
      ...(capabilities?.supports_light_quality_preset ? [{ value: "light", label: "Light" }] : []),
      { value: "none", label: "None" },
    ],
    [capabilities?.supports_light_quality_preset],
  );
  return (
    <details className="group relative">
      <summary
        aria-label={t("promptOptions")}
        className="grid size-8 cursor-pointer list-none place-items-center border border-transparent text-app-muted hover:border-app-border hover:bg-app-surface hover:text-app-text"
      >
        <Settings2 aria-hidden="true" className="size-4" />
      </summary>
      <div className="absolute top-10 right-0 z-30 w-64 space-y-4 border border-app-border bg-app-panel p-3 shadow-app-panel">
        <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
          {t("qualityTags")}
          <AppSelect
            aria-label={t("qualityTags")}
            value={draft.quality}
            options={qualityOptions}
            onValueChange={(value) => onPatch({ quality: toQualityPreset(value) })}
            onBlur={onFlush}
          />
        </label>
        {capabilities?.supports_transparent_background ? (
          <label className="flex items-center justify-between gap-3 text-sm text-app-text">
            {t("transparentBackground")}
            <input
              aria-label={t("transparentBackground")}
              type="checkbox"
              checked={draft.transparentBackground}
              onChange={(event) => onPatch({ transparentBackground: event.target.checked })}
              onBlur={onFlush}
            />
          </label>
        ) : null}
        <label
          htmlFor="generation-uc-preset"
          className="grid gap-1 text-xs font-semibold text-app-muted uppercase"
        >
          {t("ucPreset")}
          <AppSelect
            id="generation-uc-preset"
            aria-label={t("ucPreset")}
            value={draft.ucPreset}
            options={ucPresetOptions}
            onValueChange={(value) => onPatch({ ucPreset: toUcPreset(value) })}
            onBlur={onFlush}
          />
        </label>
      </div>
    </details>
  );
}

export const GenerationPromptPanel = forwardRef<
  GenerationPromptPanelHandle,
  GenerationPromptPanelProps
>(function GenerationPromptPanel(
  {
    draft,
    mainPresets,
    mainPresetsPending,
    onPatch,
    onFlush,
    onModelChange,
    modelCatalog,
    capabilities,
  },
  forwardedRef,
) {
  const { t } = useTranslation("generation");
  const [activeTab, setActiveTab] = useState<PromptTab>("positive");
  const modelOptions = useMemo(
    () =>
      modelCatalog?.map(({ model }) => ({
        value: model,
        label:
          generationModelSelectOptions.find((option) => option.value === model)?.label ?? model,
      })) ?? generationModelSelectOptions,
    [modelCatalog],
  );
  const promptTabs = useMemo(
    () => [
      { value: "positive" as const, label: t("positive") },
      { value: "negative" as const, label: t("undesiredContent") },
    ],
    [t],
  );
  const textareaRef = useRef<NaiPromptEditorHandle>(null);
  const focusPositive = useCallback(() => {
    setActiveTab("positive");
    window.requestAnimationFrame(() => textareaRef.current?.focus());
  }, []);
  useImperativeHandle(forwardedRef, () => ({ focusPositive }), [focusPositive]);

  const handlePromptChange = useCallback(
    (value: string) => {
      if (activeTab === "positive") {
        onPatch({ prompt: value });
      } else {
        onPatch({ negativePrompt: value });
      }
    },
    [activeTab, onPatch],
  );
  const handleTabChange = useCallback((value: string) => {
    setActiveTab(value === "negative" ? "negative" : "positive");
  }, []);
  const handleEditorKeyDown = useCallback((event: KeyboardEvent) => {
    if (event.ctrlKey && event.key === "Tab") {
      event.preventDefault();
      setActiveTab((current) => (current === "positive" ? "negative" : "positive"));
    }
  }, []);
  const handleModelChange = useCallback(
    (value: string) => {
      const model = toImageModel(value);
      if (onModelChange) onModelChange(model);
      else onPatch({ model });
    },
    [onModelChange, onPatch],
  );

  return (
    <section className="space-y-4 border-b border-app-border p-4">
      <label htmlFor="generation-model" className="sr-only">
        {t("model")}
      </label>
      <AppSelect
        id="generation-model"
        aria-label={t("model")}
        value={draft.model}
        options={modelOptions}
        onValueChange={handleModelChange}
        onBlur={onFlush}
      />

      <div className="space-y-2">
        <div className="flex items-center justify-between gap-2">
          <AppTabs
            label={t("promptType")}
            value={activeTab}
            tabs={promptTabs}
            onChange={handleTabChange}
          />
          <PromptOptionsMenu
            draft={draft}
            capabilities={capabilities}
            onPatch={onPatch}
            onFlush={onFlush}
          />
        </div>
        <NaiPromptEditor
          key={activeTab}
          ref={textareaRef}
          id="generation-prompt-editor"
          aria-label={activeTab === "positive" ? t("positivePrompt") : t("undesiredContent")}
          value={activeTab === "positive" ? draft.prompt : draft.negativePrompt}
          onChange={handlePromptChange}
          profile={promptProfileForModel(draft.model)}
          model={draft.model}
          onKeyDown={handleEditorKeyDown}
          onBlur={onFlush}
          minHeight={176}
        />
        <p className="text-[11px] text-app-muted">{t("promptTabsHint")}</p>
      </div>

      <GenerationPresetControl
        label={t("mainPreset")}
        noPresetLabel={t("noMainPreset")}
        libraryTitle={t("mainPresetLibrary")}
        presets={mainPresets}
        selectedPresetId={draft.mainPresetId}
        pending={mainPresetsPending}
        onSelect={(mainPresetId) => onPatch({ mainPresetId }, { persist: "immediate" })}
        onClear={() => onPatch({ mainPresetId: null }, { persist: "immediate" })}
        onApply={(preset) =>
          onPatch(
            {
              ...applyPromptPreset(preset, draft.prompt, draft.negativePrompt),
              mainPresetId: null,
            },
            { persist: "immediate" },
          )
        }
      />
    </section>
  );
});
