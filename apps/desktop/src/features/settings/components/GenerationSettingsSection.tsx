import { RotateCcw } from "lucide-react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import { SeedInput } from "@/features/generation/components/SeedInput";
import {
  findModelDescriptor,
  useImageModelCatalog,
} from "@/features/generation/data/useImageModelCatalog";
import { toQualityPreset } from "@/features/generation/model/generation-options";
import type { GenerationDefaultsDto, ModelCapabilitiesDto, WorkspaceSettingsDto } from "@/types";

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

type GenerationFieldChange = <Key extends keyof GenerationDefaultsDto>(
  key: Key,
  value: GenerationDefaultsDto[Key],
) => void;

export function GenerationSettingsSection({
  draft,
  updateDraft,
  resetSettings,
  resetting,
}: {
  draft: WorkspaceSettingsDto;
  updateDraft: (draft: WorkspaceSettingsDto) => void;
  resetSettings: () => void;
  resetting: boolean;
}) {
  const { t } = useTranslation("settings");
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
  return (
    <AppPanel variant="section" className="flex h-full min-h-0 flex-col overflow-hidden">
      <SectionHeader title={t("generation")}>
        <AppButton variant="ghost" disabled={resetting} onClick={resetSettings}>
          <RotateCcw aria-hidden="true" className="size-4" />
          {t("resetWorkspaceSettings")}
        </AppButton>
      </SectionHeader>
      <GenerationFields
        generation={generation}
        onGenerationChange={updateGeneration}
        onFieldChange={updateField}
        onSizeChange={updateSize}
      />
    </AppPanel>
  );
}

function GenerationFields({
  generation,
  onGenerationChange,
  onFieldChange,
  onSizeChange,
}: {
  generation: GenerationDefaultsDto;
  onGenerationChange: (generation: GenerationDefaultsDto) => void;
  onFieldChange: GenerationFieldChange;
  onSizeChange: (key: "width" | "height", value: number) => void;
}) {
  const { t } = useTranslation("settings");
  const modelCatalog = useImageModelCatalog();
  const capabilities = findModelDescriptor(modelCatalog.data, generation.model)?.capabilities;
  const availableModelOptions = useMemo(
    () =>
      modelCatalog.data?.map(
        ({ model }) =>
          modelSelectOptions.find((option) => option.value === model) ?? {
            value: model,
            label: model,
          },
      ) ?? modelSelectOptions,
    [modelCatalog.data],
  );
  const ucPresetOptions = useMemo(
    () =>
      ucPresetSelectOptions({
        heavy: t("ucPresetOptions.heavy"),
        light: t("ucPresetOptions.light"),
        furry_focus: t("ucPresetOptions.furry_focus"),
        human_focus: t("ucPresetOptions.human_focus"),
        none: t("ucPresetOptions.none"),
      }),
    [t],
  );
  const modelChange = useCallback(
    (value: string) => {
      const model = toImageModel(value);
      const descriptor = findModelDescriptor(modelCatalog.data, model);
      onGenerationChange({
        ...generation,
        model,
        scale: descriptor?.capabilities.default_scale ?? generation.scale,
      });
    },
    [generation, modelCatalog.data, onGenerationChange],
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

  return (
    <div className="min-h-0 flex-1 overflow-auto p-3">
      <div className="grid gap-3 md:grid-cols-3">
        <SelectField
          label={t("model")}
          value={generation.model}
          options={availableModelOptions}
          onChange={modelChange}
        />
        <NumberField label={t("width")} value={generation.size.width} onChange={widthChange} />
        <NumberField label={t("height")} value={generation.size.height} onChange={heightChange} />
        <SelectField
          label={t("sampler")}
          value={generation.sampler}
          options={samplerSelectOptions}
          onChange={samplerChange}
        />
        <SelectField
          label={t("noiseSchedule")}
          value={generation.noise_schedule}
          options={noiseScheduleSelectOptions}
          onChange={noiseChange}
        />
        <SelectField
          label={t("ucPreset")}
          value={generation.uc_preset}
          options={ucPresetOptions}
          onChange={ucPresetChange}
        />
        <NumberField label={t("steps")} value={generation.steps} onChange={stepsChange} />
        <NumberField
          label={t("scale")}
          value={generation.scale}
          step="0.1"
          onChange={scaleChange}
        />
        <NumberField label={t("samples")} value={generation.n_samples} onChange={samplesChange} />
        <SeedInput
          label={t("seed")}
          value={generation.seed}
          randomPlaceholder={t("randomSeed")}
          onChange={seedChange}
        />
        <NumberField
          label={t("cfgRescale")}
          value={generation.cfg_rescale}
          step="0.1"
          onChange={cfgChange}
        />
        <SelectField
          label={t("imageFormat")}
          value={generation.image_format ?? "default"}
          options={nullableImageFormatSelectOptions(t("novelAiDefault"))}
          onChange={formatChange}
        />
      </div>
      <GenerationCapabilityFields
        generation={generation}
        capabilities={capabilities}
        onFieldChange={onFieldChange}
      />
    </div>
  );
}

function GenerationCapabilityFields({
  generation,
  capabilities,
  onFieldChange,
}: {
  generation: GenerationDefaultsDto;
  capabilities?: ModelCapabilitiesDto;
  onFieldChange: GenerationFieldChange;
}) {
  const { t } = useTranslation("settings");
  const qualityOptions = useMemo(
    () => [
      { value: "standard", label: "Standard" },
      ...(capabilities?.supports_light_quality_preset ? [{ value: "light", label: "Light" }] : []),
      { value: "none", label: "None" },
    ],
    [capabilities?.supports_light_quality_preset],
  );
  const qualityChange = useCallback(
    (value: string) => onFieldChange("quality", toQualityPreset(value)),
    [onFieldChange],
  );
  const transparentChange = useCallback(
    (value: boolean) => onFieldChange("transparent_background", value),
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
    <div className="mt-4 grid gap-2 border-t border-app-border pt-4 md:grid-cols-3">
      <SelectField
        label={t("quality")}
        value={generation.quality}
        options={qualityOptions}
        onChange={qualityChange}
      />
      {capabilities?.supports_transparent_background ? (
        <CheckboxField
          label={t("transparentBackground")}
          checked={generation.transparent_background}
          onChange={transparentChange}
        />
      ) : null}
      {capabilities?.supports_variety_boost !== false ? (
        <CheckboxField
          label={t("varietyBoost")}
          checked={generation.variety_boost}
          onChange={varietyChange}
        />
      ) : null}
      <CheckboxField
        label={t("strictMode")}
        checked={generation.strict_mode}
        onChange={strictChange}
      />
    </div>
  );
}
