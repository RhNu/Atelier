import {
  generationImageFormatOptions,
  generationModelSelectOptions,
  generationNoiseScheduleOptions,
  generationSamplerSelectOptions,
  generationUcPresetOptions,
  toImageFormat,
  toImageModel,
  toNoiseSchedule,
  toSampler,
  toSelectOptions,
  toUcPreset,
} from "@/features/generation/model/generation-options";
import type { UcPresetDto } from "@/types";

export const modelSelectOptions = generationModelSelectOptions;
export const samplerSelectOptions = generationSamplerSelectOptions;
export const noiseScheduleSelectOptions = toSelectOptions(generationNoiseScheduleOptions);
export function ucPresetSelectOptions(labels?: Partial<Record<UcPresetDto, string>>) {
  return toSelectOptions(generationUcPresetOptions, labels);
}
export const imageFormatSelectOptions = toSelectOptions(generationImageFormatOptions);

export function nullableImageFormatSelectOptions(defaultLabel: string) {
  return [{ value: "default", label: defaultLabel }, ...imageFormatSelectOptions];
}

export { toImageFormat, toImageModel, toNoiseSchedule, toSampler, toUcPreset };
