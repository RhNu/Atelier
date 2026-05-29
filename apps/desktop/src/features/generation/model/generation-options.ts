import type {
  ImageFormatDto,
  ImageModelDto,
  NoiseScheduleDto,
  SamplerDto,
  UcPresetDto,
} from "../../../types";

export const generationModelOptions: ReadonlyArray<ImageModelDto> = [
  "nai-diffusion-4-5-full",
  "nai-diffusion-4-5-curated",
  "nai-diffusion-4-full",
  "nai-diffusion-4-curated",
  "nai-diffusion-3",
  "nai-diffusion-3-furry",
];

export const generationSamplerOptions: ReadonlyArray<SamplerDto> = [
  "k_euler",
  "k_euler_ancestral",
  "k_dpm2",
  "k_dpm2_ancestral",
  "k_dpmpp2m",
  "k_dpmpp2s_ancestral",
  "k_dpmpp_sde",
  "ddim",
];

export const generationNoiseScheduleOptions: ReadonlyArray<NoiseScheduleDto> = [
  "karras",
  "exponential",
  "polyexponential",
];

export const generationUcPresetOptions: ReadonlyArray<UcPresetDto> = [
  "heavy",
  "light",
  "furry_focus",
  "human_focus",
  "none",
];

export const generationImageFormatOptions: ReadonlyArray<ImageFormatDto> = ["png", "webp"];

export function toSelectOptions(values: ReadonlyArray<string>) {
  return values.map((value) => ({ value, label: value }));
}

export function toImageFormat(value: string): ImageFormatDto | null {
  if (value === "png" || value === "webp") {
    return value;
  }
  return null;
}

export function toImageModel(value: string): ImageModelDto {
  switch (value) {
    case "nai-diffusion-4-5-full":
    case "nai-diffusion-4-5-curated":
    case "nai-diffusion-4-full":
    case "nai-diffusion-4-curated":
    case "nai-diffusion-3":
    case "nai-diffusion-3-furry":
      return value;
    default:
      return "nai-diffusion-4-5-full";
  }
}

export function toSampler(value: string): SamplerDto {
  switch (value) {
    case "k_euler":
    case "k_euler_ancestral":
    case "k_dpm2":
    case "k_dpm2_ancestral":
    case "k_dpmpp2m":
    case "k_dpmpp2s_ancestral":
    case "k_dpmpp_sde":
    case "ddim":
      return value;
    default:
      return "k_euler";
  }
}

export function toNoiseSchedule(value: string): NoiseScheduleDto {
  switch (value) {
    case "karras":
    case "exponential":
    case "polyexponential":
      return value;
    default:
      return "karras";
  }
}

export function toUcPreset(value: string): UcPresetDto {
  switch (value) {
    case "heavy":
    case "light":
    case "furry_focus":
    case "human_focus":
    case "none":
      return value;
    default:
      return "heavy";
  }
}
