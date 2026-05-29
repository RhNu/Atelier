import type {
  GenerateImageRequestDto,
  GenerationWorkRequestDto,
  ImageFormatDto,
  ImageModelDto,
  ImageSizeDto,
  NoiseScheduleDto,
  SamplerDto,
  SubmitGenerationRequestDto,
  UcPresetDto,
  WorkspaceSettingsDto,
} from "../../../types";

export type GenerationDraft = {
  prompt: string;
  negativePrompt: string;
  model: ImageModelDto;
  size: ImageSizeDto;
  quality: boolean;
  ucPreset: UcPresetDto;
  steps: number;
  scale: number;
  sampler: SamplerDto;
  noiseSchedule: NoiseScheduleDto;
  seed: number;
  nSamples: number;
  cfgRescale: number;
  varietyBoost: boolean;
  imageFormat: ImageFormatDto | null;
  strictMode: boolean;
  streamEnabled: boolean;
};

export type GenerationRunIds = {
  batchId: string;
  jobId: string;
};

export function createGenerationDraft(settings: WorkspaceSettingsDto): GenerationDraft {
  const defaults = settings.generation;

  return {
    prompt: "",
    negativePrompt: "",
    model: defaults.model,
    size: { ...defaults.size },
    quality: defaults.quality,
    ucPreset: defaults.uc_preset,
    steps: defaults.steps,
    scale: defaults.scale,
    sampler: defaults.sampler,
    noiseSchedule: defaults.noise_schedule,
    seed: defaults.seed,
    nSamples: defaults.n_samples,
    cfgRescale: defaults.cfg_rescale,
    varietyBoost: defaults.variety_boost,
    imageFormat: defaults.image_format,
    strictMode: defaults.strict_mode,
    streamEnabled: true,
  };
}

export function canSubmitGenerationDraft(draft: GenerationDraft): boolean {
  return draft.prompt.trim().length > 0;
}

export function createGenerationRunIds(): GenerationRunIds {
  return {
    batchId: `generation-${createId()}`,
    jobId: `job-${createId()}`,
  };
}

export function buildSubmitGenerationRequest(
  draft: GenerationDraft,
  ids: GenerationRunIds = createGenerationRunIds(),
): SubmitGenerationRequestDto {
  const base = buildBaseGenerateRequest(draft);
  const work: GenerationWorkRequestDto = draft.streamEnabled
    ? {
        kind: "stream",
        request: {
          base,
          stream: "sse",
        },
      }
    : {
        kind: "image",
        request: base,
      };

  return {
    batch_id: ids.batchId,
    job_id: ids.jobId,
    work,
    context: {
      request_count: 1,
      pending_vibe_encode_count: 0,
      is_opus: false,
    },
  };
}

function buildBaseGenerateRequest(draft: GenerationDraft): GenerateImageRequestDto {
  return {
    prompt: draft.prompt,
    model: draft.model,
    size: { ...draft.size },
    negative_prompt: normalizeOptionalText(draft.negativePrompt),
    quality: draft.quality,
    uc_preset: draft.ucPreset,
    steps: draft.steps,
    scale: draft.scale,
    sampler: draft.sampler,
    noise_schedule: draft.noiseSchedule,
    seed: draft.seed,
    n_samples: draft.nSamples,
    cfg_rescale: draft.cfgRescale,
    variety_boost: draft.varietyBoost,
    strict_mode: draft.strictMode,
    i2i: null,
    controlnet: null,
    character_references: null,
    characters: null,
    use_coords: null,
    image_format: draft.imageFormat,
  };
}

function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? value : null;
}

function createId(): string {
  if (globalThis.crypto && "randomUUID" in globalThis.crypto) {
    return globalThis.crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
