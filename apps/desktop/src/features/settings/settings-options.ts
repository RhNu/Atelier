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

export const modelSelectOptions = generationModelSelectOptions;
export const samplerSelectOptions = generationSamplerSelectOptions;
export const noiseScheduleSelectOptions = toSelectOptions(generationNoiseScheduleOptions);
export const ucPresetSelectOptions = toSelectOptions(generationUcPresetOptions);
export const imageFormatSelectOptions = toSelectOptions(generationImageFormatOptions);

export function nullableImageFormatSelectOptions(defaultLabel: string) {
  return [{ value: "default", label: defaultLabel }, ...imageFormatSelectOptions];
}

export { toImageFormat, toImageModel, toNoiseSchedule, toSampler, toUcPreset };
