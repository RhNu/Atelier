import { useCallback } from "react";

import { AppPanel } from "../../../components/ui";
import type { GenerationDraft } from "../model/generation-draft";
import {
  generationImageFormatOptions,
  generationModelOptions,
  generationNoiseScheduleOptions,
  generationSamplerOptions,
  generationUcPresetOptions,
  toSelectOptions,
} from "../model/generation-options";
import { BooleanField, NumberField, SelectField, type SelectOption } from "./GenerationFormFields";
import { useGenerationParamHandlers } from "./useGenerationParamHandlers";

type GenerationParamsPanelProps = {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  onPatchSize: (patch: Partial<GenerationDraft["size"]>) => void;
};

const MODEL_OPTIONS = toSelectOptions(generationModelOptions);
const SAMPLER_OPTIONS = toSelectOptions(generationSamplerOptions);
const NOISE_SCHEDULE_OPTIONS = toSelectOptions(generationNoiseScheduleOptions);
const UC_PRESET_OPTIONS = toSelectOptions(generationUcPresetOptions);
const IMAGE_FORMAT_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "default", label: "NovelAI default" },
  ...toSelectOptions(generationImageFormatOptions),
];
const SEED_MODE_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "random", label: "Random" },
  { value: "fixed", label: "Fixed" },
];
const SIZE_PRESETS: ReadonlyArray<{ label: string; width: number; height: number }> = [
  { label: "Portrait", width: 832, height: 1216 },
  { label: "Landscape", width: 1216, height: 832 },
  { label: "Square", width: 1024, height: 1024 },
  { label: "Large portrait", width: 1024, height: 1536 },
  { label: "Large landscape", width: 1536, height: 1024 },
  { label: "Small portrait", width: 512, height: 768 },
  { label: "Small landscape", width: 768, height: 512 },
  { label: "Small square", width: 640, height: 640 },
];

export function GenerationParamsPanel({ draft, onPatch, onPatchSize }: GenerationParamsPanelProps) {
  const handlers = useGenerationParamHandlers({ onPatch, onPatchSize });

  return (
    <AppPanel className="min-h-0 overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Generation Parameters</h2>
      </header>
      <div className="grid gap-3 p-3">
        <SelectField
          label="Model"
          value={draft.model}
          options={MODEL_OPTIONS}
          onChange={handlers.handleModelChange}
        />
        <div className="grid grid-cols-2 gap-2">
          {SIZE_PRESETS.map((preset) => (
            <SizePresetButton key={preset.label} preset={preset} onPatchSize={onPatchSize} />
          ))}
        </div>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label="Width"
            value={draft.size.width}
            min={64}
            step={64}
            onChange={handlers.handleWidthChange}
          />
          <NumberField
            label="Height"
            value={draft.size.height}
            min={64}
            step={64}
            onChange={handlers.handleHeightChange}
          />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label="Steps"
            value={draft.steps}
            min={1}
            max={50}
            onChange={handlers.handleStepsChange}
          />
          <NumberField
            label="Scale"
            value={draft.scale}
            min={0}
            max={10}
            step={0.1}
            onChange={handlers.handleScaleChange}
          />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label="Seed mode"
            value={draft.seedMode}
            options={SEED_MODE_OPTIONS}
            onChange={handlers.handleSeedModeChange}
          />
          <NumberField label="Seed" value={draft.seed} onChange={handlers.handleSeedChange} />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label="Samples"
            value={draft.nSamples}
            min={1}
            max={4}
            onChange={handlers.handleSamplesChange}
          />
          <NumberField
            label="Requests"
            value={draft.requestCount}
            min={1}
            max={8}
            onChange={handlers.handleRequestCountChange}
          />
        </div>
        <SelectField
          label="Sampler"
          value={draft.sampler}
          options={SAMPLER_OPTIONS}
          onChange={handlers.handleSamplerChange}
        />
        <SelectField
          label="Noise schedule"
          value={draft.noiseSchedule}
          options={NOISE_SCHEDULE_OPTIONS}
          onChange={handlers.handleNoiseScheduleChange}
        />
        <SelectField
          label="UC preset"
          value={draft.ucPreset}
          options={UC_PRESET_OPTIONS}
          onChange={handlers.handleUcPresetChange}
        />
        <SelectField
          label="Output format"
          value={draft.imageFormat ?? "default"}
          options={IMAGE_FORMAT_OPTIONS}
          onChange={handlers.handleImageFormatChange}
        />
        <NumberField
          label="CFG rescale"
          value={draft.cfgRescale}
          min={0}
          max={1}
          step={0.01}
          onChange={handlers.handleCfgRescaleChange}
        />
        <div className="grid grid-cols-2 gap-2 text-sm text-app-text">
          <BooleanField
            label="Quality tags"
            checked={draft.quality}
            onChange={handlers.handleQualityChange}
          />
          <BooleanField
            label="Variety boost"
            checked={draft.varietyBoost}
            onChange={handlers.handleVarietyBoostChange}
          />
          <BooleanField
            label="Strict mode"
            checked={draft.strictMode}
            onChange={handlers.handleStrictModeChange}
          />
          <BooleanField
            label="Streaming preview"
            checked={draft.streamEnabled}
            onChange={handlers.handleStreamEnabledChange}
          />
        </div>
      </div>
    </AppPanel>
  );
}

function SizePresetButton({
  preset,
  onPatchSize,
}: {
  preset: { label: string; width: number; height: number };
  onPatchSize: (patch: Partial<GenerationDraft["size"]>) => void;
}) {
  const handleClick = useCallback(() => {
    onPatchSize({ width: preset.width, height: preset.height });
  }, [onPatchSize, preset.height, preset.width]);

  return (
    <button
      type="button"
      className="border border-app-border bg-app-surface/70 px-2 py-1 text-left text-xs text-app-text hover:border-brand-400"
      onClick={handleClick}
    >
      {preset.label}
    </button>
  );
}
