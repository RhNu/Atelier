/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Settings2 } from "lucide-react";
import {
  forwardRef,
  useCallback,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
} from "react";

import { AppSelect, AppTabs } from "../../../components/ui";
import type { PromptPresetDto } from "../../../types";
import type { GenerationDraft } from "../model/generation-draft";
import {
  generationModelOptions,
  generationUcPresetOptions,
  toImageModel,
  toUcPreset,
  toSelectOptions,
} from "../model/generation-options";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { PromptCompletionTextarea } from "./prompt-completion";

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
};

const MODEL_OPTIONS = toSelectOptions(generationModelOptions);
const UC_PRESET_OPTIONS = toSelectOptions(generationUcPresetOptions);
const PROMPT_TABS = [
  { value: "positive", label: "Positive" },
  { value: "negative", label: "Undesired Content" },
] as const;

export const GenerationPromptPanel = forwardRef<
  GenerationPromptPanelHandle,
  GenerationPromptPanelProps
>(function GenerationPromptPanel(
  { draft, mainPresets, mainPresetsPending, onPatch, onFlush },
  forwardedRef,
) {
  const [activeTab, setActiveTab] = useState<PromptTab>("positive");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const mainPresetOptions = useMemo(
    () => [
      {
        value: "",
        label: mainPresetsPending ? "Loading presets" : "No main preset",
      },
      ...mainPresets
        .filter((preset) => preset.enabled)
        .map((preset) => ({ value: preset.preset_id, label: preset.name })),
    ],
    [mainPresets, mainPresetsPending],
  );

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
  const handleEditorKeyDown = useCallback((event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.ctrlKey && event.key === "Tab") {
      event.preventDefault();
      setActiveTab((current) => (current === "positive" ? "negative" : "positive"));
    }
  }, []);

  return (
    <section className="space-y-4 border-b border-app-border p-4">
      <label
        htmlFor="generation-model"
        className="grid gap-1.5 text-xs font-semibold text-app-muted uppercase"
      >
        Model
        <AppSelect
          id="generation-model"
          aria-label="Model"
          value={draft.model}
          options={MODEL_OPTIONS}
          onChange={(event) => onPatch({ model: toImageModel(event.target.value) })}
          onBlur={onFlush}
        />
      </label>

      <div className="space-y-2">
        <div className="flex items-center justify-between gap-2">
          <AppTabs
            label="Prompt type"
            value={activeTab}
            tabs={PROMPT_TABS}
            onChange={handleTabChange}
          />
          <details className="group relative">
            <summary
              aria-label="Prompt options"
              className="grid size-8 cursor-pointer list-none place-items-center border border-transparent text-app-muted hover:border-app-border hover:bg-app-surface hover:text-app-text"
            >
              <Settings2 aria-hidden="true" className="size-4" />
            </summary>
            <div className="absolute top-10 right-0 z-30 w-64 space-y-4 border border-app-border bg-app-panel p-3 shadow-app-panel">
              <label className="flex items-center justify-between gap-3 text-sm text-app-text">
                Quality tags
                <input
                  aria-label="Quality tags"
                  type="checkbox"
                  checked={draft.quality}
                  onChange={(event) => onPatch({ quality: event.target.checked })}
                  onBlur={onFlush}
                />
              </label>
              <label
                htmlFor="generation-uc-preset"
                className="grid gap-1 text-xs font-semibold text-app-muted uppercase"
              >
                UC preset
                <AppSelect
                  id="generation-uc-preset"
                  aria-label="UC preset"
                  value={draft.ucPreset}
                  options={UC_PRESET_OPTIONS}
                  onChange={(event) => onPatch({ ucPreset: toUcPreset(event.target.value) })}
                  onBlur={onFlush}
                />
              </label>
            </div>
          </details>
        </div>
        <PromptCompletionTextarea
          ref={textareaRef}
          id="generation-prompt-editor"
          aria-label={activeTab === "positive" ? "Positive prompt" : "Undesired content"}
          value={activeTab === "positive" ? draft.prompt : draft.negativePrompt}
          onChange={handlePromptChange}
          onKeyDown={handleEditorKeyDown}
          onBlur={onFlush}
          className="min-h-44 resize-y border border-app-border bg-black/20 p-3 text-sm text-app-text outline-none focus:border-brand-400"
        />
        <p className="text-[11px] text-app-muted">Ctrl+Tab switches prompt tabs.</p>
      </div>

      <label
        htmlFor="generation-main-preset"
        className="grid gap-1.5 text-xs font-semibold text-app-muted uppercase"
      >
        Main preset
        <AppSelect
          id="generation-main-preset"
          aria-label="Main preset"
          value={draft.mainPresetId ?? ""}
          options={mainPresetOptions}
          onChange={(event: ChangeEvent<HTMLSelectElement>) =>
            onPatch({ mainPresetId: event.target.value || null })
          }
          onBlur={onFlush}
        />
      </label>
    </section>
  );
});
