import type {
  ImageFormatDto,
  ImageModelDto,
  NoiseScheduleDto,
  QualityPresetDto,
  SamplerDto,
  UcPresetDto,
} from "@/types";

export const generationModelOptions: ReadonlyArray<ImageModelDto> = [
  "nai-diffusion-5-full",
  "nai-diffusion-5-curated",
  "nai-diffusion-4-5-full",
  "nai-diffusion-4-5-curated",
  "nai-diffusion-4-full",
  "nai-diffusion-4-curated",
  "nai-diffusion-3",
  "nai-diffusion-furry-3",
];

export const generationSamplerOptions: ReadonlyArray<SamplerDto> = [
  "k_euler",
  "k_euler_ancestral",
  "k_dpm2",
  "k_dpm2_ancestral",
  "k_dpmpp2m",
  "k_dpmpp2m_sde",
  "k_dpmpp2s_ancestral",
  "k_dpmpp_sde",
  "ddim",
  "ddim_v3",
];

export const generationNoiseScheduleOptions: ReadonlyArray<NoiseScheduleDto> = [
  "native",
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

export const generationModelDisplayNames: Record<ImageModelDto, string> = {
  "nai-diffusion-5-full": "NAI Diffusion 5 Full",
  "nai-diffusion-5-curated": "NAI Diffusion 5 Curated",
  "nai-diffusion-4-5-full": "NAI Diffusion 4.5 Full",
  "nai-diffusion-4-5-curated": "NAI Diffusion 4.5 Curated",
  "nai-diffusion-4-full": "NAI Diffusion 4 Full",
  "nai-diffusion-4-curated": "NAI Diffusion 4 Curated",
  "nai-diffusion-3": "NAI Diffusion 3",
  "nai-diffusion-furry-3": "NAI Diffusion 3 Furry",
};

export const generationSamplerDisplayNames: Record<SamplerDto, string> = {
  k_euler: "Euler",
  k_euler_ancestral: "Euler A",
  k_dpm2: "DPM2",
  k_dpm2_ancestral: "DPM2 A",
  k_dpmpp2m: "DPM++ 2M",
  k_dpmpp2m_sde: "DPM++ 2M SDE",
  k_dpmpp2s_ancestral: "DPM++ 2S A",
  k_dpmpp_sde: "DPM++ SDE",
  ddim: "DDIM",
  ddim_v3: "DDIM V3",
};

export function toSelectOptions<Value extends string>(
  values: ReadonlyArray<Value>,
  labels?: Partial<Record<Value, string>>,
) {
  return values.map((value) => ({ value, label: labels?.[value] ?? value }));
}

export const generationModelSelectOptions = toSelectOptions(
  generationModelOptions,
  generationModelDisplayNames,
);
export const generationSamplerSelectOptions = toSelectOptions(
  generationSamplerOptions,
  generationSamplerDisplayNames,
);

export function toImageFormat(value: string): ImageFormatDto | null {
  if (value === "png" || value === "webp") {
    return value;
  }
  return null;
}

export function toImageModel(value: string): ImageModelDto {
  switch (value) {
    case "nai-diffusion-5-full":
    case "nai-diffusion-5-curated":
    case "nai-diffusion-4-5-full":
    case "nai-diffusion-4-5-curated":
    case "nai-diffusion-4-full":
    case "nai-diffusion-4-curated":
    case "nai-diffusion-3":
    case "nai-diffusion-furry-3":
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
    case "k_dpmpp2m_sde":
    case "k_dpmpp2s_ancestral":
    case "k_dpmpp_sde":
    case "ddim":
    case "ddim_v3":
      return value;
    default:
      return "k_euler";
  }
}

export function toNoiseSchedule(value: string): NoiseScheduleDto {
  switch (value) {
    case "native":
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

export function toQualityPreset(value: string): QualityPresetDto {
  switch (value) {
    case "standard":
    case "light":
    case "none":
      return value;
    default:
      return "standard";
  }
}
