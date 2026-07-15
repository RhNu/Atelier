import { RotateCcw, Save } from "lucide-react";
import { useCallback } from "react";

import { AppButton, AppPanel } from "../../../components/ui";
import type { GenerationDefaultsDto, WorkspaceSettingsDto } from "../../../types";
import {
  modelSelectOptions,
  noiseScheduleSelectOptions,
  nullableImageFormatSelectOptions,
  samplerSelectOptions,
  toImageFormat,
  toImageModel,
  toNoiseSchedule,
  toSampler,
  toUcPreset,
  ucPresetSelectOptions,
} from "../settings-options";
import { CheckboxField, NumberField, SectionHeader, SelectField } from "./SettingsControls";

export function GenerationSettingsSection({
  draft,
  updateDraft,
  saveSettings,
  resetSettings,
  saving,
  resetting,
  commandError,
}: {
  draft: WorkspaceSettingsDto;
  updateDraft: (draft: WorkspaceSettingsDto) => void;
  saveSettings: (settings: WorkspaceSettingsDto) => void;
  resetSettings: () => void;
  saving: boolean;
  resetting: boolean;
  commandError: string | null;
}) {
  const generation = draft.generation;
  const updateGeneration = useCallback(
    (nextGeneration: GenerationDefaultsDto) => {
      updateDraft({ ...draft, generation: nextGeneration });
    },
    [draft, updateDraft],
  );
  const updateField = useCallback(
    <Key extends keyof GenerationDefaultsDto>(key: Key, value: GenerationDefaultsDto[Key]) => {
      updateGeneration({ ...generation, [key]: value });
    },
    [generation, updateGeneration],
  );
  const updateSize = useCallback(
    (key: "width" | "height", value: number) => {
      updateGeneration({ ...generation, size: { ...generation.size, [key]: value } });
    },
    [generation, updateGeneration],
  );
  const save = useCallback(() => {
    saveSettings(draft);
  }, [draft, saveSettings]);

  return (
    <AppPanel variant="section" className="flex h-full min-h-0 flex-col overflow-hidden">
      <SectionHeader
        kicker="Generation"
        title="Generation Defaults"
        description="Workspace-local prefill values for NovelAI image requests."
      >
        <AppButton variant="ghost" disabled={resetting} onClick={resetSettings}>
          <RotateCcw aria-hidden="true" className="size-4" />
          Reset workspace settings
        </AppButton>
        <AppButton disabled={saving} onClick={save}>
          <Save aria-hidden="true" className="size-4" />
          Save generation defaults
        </AppButton>
      </SectionHeader>
      <GenerationFields
        generation={generation}
        onFieldChange={updateField}
        onSizeChange={updateSize}
      />
      {commandError ? <SettingsCommandError message={commandError} /> : null}
    </AppPanel>
  );
}

function SettingsCommandError({ message }: { message: string }) {
  return (
    <p className="border-t border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
      {message}
    </p>
  );
}

function GenerationFields({
  generation,
  onFieldChange,
  onSizeChange,
}: {
  generation: GenerationDefaultsDto;
  onFieldChange: <Key extends keyof GenerationDefaultsDto>(
    key: Key,
    value: GenerationDefaultsDto[Key],
  ) => void;
  onSizeChange: (key: "width" | "height", value: number) => void;
}) {
  const modelChange = useCallback(
    (value: string) => onFieldChange("model", toImageModel(value)),
    [onFieldChange],
  );
  const widthChange = useCallback((value: number) => onSizeChange("width", value), [onSizeChange]);
  const heightChange = useCallback(
    (value: number) => onSizeChange("height", value),
    [onSizeChange],
  );
  const samplerChange = useCallback(
    (value: string) => onFieldChange("sampler", toSampler(value)),
    [onFieldChange],
  );
  const noiseChange = useCallback(
    (value: string) => onFieldChange("noise_schedule", toNoiseSchedule(value)),
    [onFieldChange],
  );
  const ucPresetChange = useCallback(
    (value: string) => onFieldChange("uc_preset", toUcPreset(value)),
    [onFieldChange],
  );
  const stepsChange = useCallback(
    (value: number) => onFieldChange("steps", value),
    [onFieldChange],
  );
  const scaleChange = useCallback(
    (value: number) => onFieldChange("scale", value),
    [onFieldChange],
  );
  const samplesChange = useCallback(
    (value: number) => onFieldChange("n_samples", value),
    [onFieldChange],
  );
  const seedChange = useCallback((value: number) => onFieldChange("seed", value), [onFieldChange]);
  const cfgChange = useCallback(
    (value: number) => onFieldChange("cfg_rescale", value),
    [onFieldChange],
  );
  const formatChange = useCallback(
    (value: string) => onFieldChange("image_format", toImageFormat(value)),
    [onFieldChange],
  );
  const qualityChange = useCallback(
    (value: boolean) => onFieldChange("quality", value),
    [onFieldChange],
  );
  const varietyChange = useCallback(
    (value: boolean) => onFieldChange("variety_boost", value),
    [onFieldChange],
  );
  const strictChange = useCallback(
    (value: boolean) => onFieldChange("strict_mode", value),
    [onFieldChange],
  );

  return (
    <div className="min-h-0 flex-1 overflow-auto p-3">
      <div className="grid gap-3 md:grid-cols-3">
        <SelectField
          label="Model"
          value={generation.model}
          options={modelSelectOptions}
          onChange={modelChange}
        />
        <NumberField label="Width" value={generation.size.width} onChange={widthChange} />
        <NumberField label="Height" value={generation.size.height} onChange={heightChange} />
        <SelectField
          label="Sampler"
          value={generation.sampler}
          options={samplerSelectOptions}
          onChange={samplerChange}
        />
        <SelectField
          label="Noise schedule"
          value={generation.noise_schedule}
          options={noiseScheduleSelectOptions}
          onChange={noiseChange}
        />
        <SelectField
          label="UC preset"
          value={generation.uc_preset}
          options={ucPresetSelectOptions}
          onChange={ucPresetChange}
        />
        <NumberField label="Steps" value={generation.steps} onChange={stepsChange} />
        <NumberField label="Scale" value={generation.scale} step="0.1" onChange={scaleChange} />
        <NumberField label="Samples" value={generation.n_samples} onChange={samplesChange} />
        <NumberField label="Seed" value={generation.seed} onChange={seedChange} />
        <NumberField
          label="CFG rescale"
          value={generation.cfg_rescale}
          step="0.1"
          onChange={cfgChange}
        />
        <SelectField
          label="Image format"
          value={generation.image_format ?? "default"}
          options={nullableImageFormatSelectOptions}
          onChange={formatChange}
        />
      </div>
      <div className="mt-4 grid gap-2 border-t border-app-border pt-4 md:grid-cols-3">
        <CheckboxField label="Quality" checked={generation.quality} onChange={qualityChange} />
        <CheckboxField
          label="Variety boost"
          checked={generation.variety_boost}
          onChange={varietyChange}
        />
        <CheckboxField
          label="Strict mode"
          checked={generation.strict_mode}
          onChange={strictChange}
        />
      </div>
    </div>
  );
}
