import type {
  CharacterReferenceTypeDto,
  CharacterDto,
  GenerateImageRequestDto,
  GenerationPlanContextDto,
  GenerationEstimateRequestDto,
  GenerationWorkRequestDto,
  ImageFormatDto,
  ImageInputDto,
  ImageModelDto,
  ImageSizeDto,
  NoiseScheduleDto,
  ResourceRefDto,
  SamplerDto,
  SubmitGenerationBatchRequestDto,
  UcPresetDto,
  WorkspaceSettingsDto,
} from "../../../types";

export type GenerationSeedMode = "random" | "fixed";
export type GenerationCharacterPositionMode = "global" | "manual";

export type GenerationI2iDraft = {
  image: ResourceRefDto;
  mask: ResourceRefDto | null;
  strength: number;
  noise: number;
};

export type GenerationVibeSlotDraft = {
  id: string;
  encoding: ResourceRefDto;
  vibeId?: string | null;
  informationExtracted: number;
  strength: number;
  displayName: string;
  sourceImage: ResourceRefDto | null;
  sourceSha256: string | null;
};

export type GenerationPreciseReferenceDraft = {
  id: string;
  image: ResourceRefDto;
  referenceType: CharacterReferenceTypeDto;
  fidelity: number;
  strength: number;
  displayName: string;
};

export type GenerationCharacterDraft = {
  id: string;
  prompt: string;
  negativePrompt: string;
  enabled: boolean;
  position: { x: number; y: number };
};

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
  seedMode: GenerationSeedMode;
  seed: number;
  nSamples: number;
  requestCount: number;
  cfgRescale: number;
  varietyBoost: boolean;
  imageFormat: ImageFormatDto | null;
  strictMode: boolean;
  streamEnabled: boolean;
  i2i: GenerationI2iDraft | null;
  vibe: {
    enabled: boolean;
    strength: number;
    slots: GenerationVibeSlotDraft[];
  };
  preciseReferences: GenerationPreciseReferenceDraft[];
  characters: GenerationCharacterDraft[];
  characterPositionMode: GenerationCharacterPositionMode;
};

export type GenerationRunIds = {
  batchId: string;
  jobIds: string[];
};

export type GenerationPlanOptions = {
  isOpus?: boolean;
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
    seedMode: defaults.seed === 0 ? "random" : "fixed",
    seed: defaults.seed,
    nSamples: defaults.n_samples,
    requestCount: 1,
    cfgRescale: defaults.cfg_rescale,
    varietyBoost: defaults.variety_boost,
    imageFormat: defaults.image_format,
    strictMode: defaults.strict_mode,
    streamEnabled: true,
    i2i: null,
    vibe: { enabled: false, strength: 1, slots: [] },
    preciseReferences: [],
    characters: [],
    characterPositionMode: "global",
  };
}

export function canSubmitGenerationDraft(draft: GenerationDraft): boolean {
  return draft.prompt.trim().length > 0;
}

export function createGenerationRunIds(requestCount = 1): GenerationRunIds {
  return {
    batchId: `generation-${createId()}`,
    jobIds: Array.from({ length: clampInteger(requestCount, 1, 8) }, () => `job-${createId()}`),
  };
}

export function buildSubmitGenerationBatchRequest(
  draft: GenerationDraft,
  ids: GenerationRunIds = createGenerationRunIds(draft.requestCount),
  options: GenerationPlanOptions = {},
): SubmitGenerationBatchRequestDto {
  const jobIds = ids.jobIds.length > 0 ? ids.jobIds : createGenerationRunIds(1).jobIds;
  const jobs = jobIds.map((jobId) => ({
    job_id: jobId,
    work: buildGenerationWorkRequest(draft),
  }));

  return {
    batch_id: ids.batchId,
    jobs,
    context: buildGenerationPlanContext(draft, jobs.length, options.isOpus ?? false),
  };
}

export function buildGenerationEstimateRequest(
  draft: GenerationDraft,
  options: GenerationPlanOptions = {},
): GenerationEstimateRequestDto {
  return {
    request: buildBaseGenerateRequest(draft),
    context: buildGenerationPlanContext(draft, draft.requestCount, options.isOpus ?? false),
  };
}

export function buildGenerationEstimateCacheKey(
  draft: GenerationDraft,
  options: GenerationPlanOptions = {},
) {
  const preciseReferences = buildPreciseReferences(draft);
  return {
    model: draft.model,
    size: { ...draft.size },
    steps: draft.steps,
    nSamples: draft.nSamples,
    requestCount: draft.requestCount,
    strictMode: draft.strictMode,
    hasI2i: Boolean(draft.i2i),
    i2iStrength: draft.i2i?.strength ?? null,
    preciseReferenceCount: preciseReferences?.length ?? 0,
    vibeSlotCount: preciseReferences ? 0 : (buildControlNet(draft)?.images.length ?? 0),
    pendingVibeEncodeCount: buildPendingVibeEncodeCount(draft),
    isOpus: options.isOpus ?? false,
  };
}

function buildGenerationWorkRequest(draft: GenerationDraft): GenerationWorkRequestDto {
  const base = buildBaseGenerateRequest(draft);
  return draft.streamEnabled
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
}

function buildGenerationPlanContext(
  draft: GenerationDraft,
  requestCount: number,
  isOpus: boolean,
): GenerationPlanContextDto {
  return {
    request_count: clampInteger(requestCount, 1, 8),
    pending_vibe_encode_count: buildPendingVibeEncodeCount(draft),
    is_opus: isOpus,
  };
}

function buildBaseGenerateRequest(draft: GenerationDraft): GenerateImageRequestDto {
  const preciseReferences = buildPreciseReferences(draft);
  const characters = buildCharacters(draft);

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
    seed: draft.seedMode === "fixed" ? draft.seed : 0,
    n_samples: draft.nSamples,
    cfg_rescale: draft.cfgRescale,
    variety_boost: draft.varietyBoost,
    strict_mode: draft.strictMode,
    i2i: draft.i2i
      ? {
          image: resourceImageInput(draft.i2i.image),
          strength: draft.i2i.strength,
          noise: draft.i2i.noise,
          mask: draft.i2i.mask ? resourceImageInput(draft.i2i.mask) : null,
        }
      : null,
    controlnet: preciseReferences ? null : buildControlNet(draft),
    character_references: preciseReferences,
    characters,
    use_coords: characters ? draft.characterPositionMode === "manual" : null,
    image_format: draft.imageFormat,
  };
}

function buildCharacters(draft: GenerationDraft): CharacterDto[] | null {
  const characters = draft.characters
    .filter((character) => character.enabled && character.prompt.trim().length > 0)
    .map((character) => ({
      prompt: character.prompt,
      negative_prompt: normalizeOptionalText(character.negativePrompt),
      position:
        draft.characterPositionMode === "manual" ? { ...character.position } : { x: 0.5, y: 0.5 },
      enabled: true,
    }));
  return characters.length ? characters : null;
}

function buildPendingVibeEncodeCount(draft: GenerationDraft): number {
  return draft.vibe.enabled ? draft.vibe.slots.filter((slot) => !slot.encoding).length : 0;
}

function buildControlNet(draft: GenerationDraft): GenerateImageRequestDto["controlnet"] {
  const images = draft.vibe.enabled
    ? draft.vibe.slots
        .filter((slot) => Boolean(slot.encoding))
        .map((slot) => ({
          encoding: slot.encoding,
          info_extracted: slot.informationExtracted,
          strength: slot.strength,
        }))
    : [];

  return images.length ? { images, strength: draft.vibe.strength } : null;
}

function buildPreciseReferences(
  draft: GenerationDraft,
): GenerateImageRequestDto["character_references"] {
  if (draft.preciseReferences.length === 0) {
    return null;
  }
  return draft.preciseReferences.map((reference) => ({
    image: resourceImageInput(reference.image),
    reference_type: reference.referenceType,
    fidelity: reference.fidelity,
    strength: reference.strength,
  }));
}

function resourceImageInput(resource: ResourceRefDto): ImageInputDto {
  return { kind: "resource_ref", resource };
}

function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function clampInteger(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.min(max, Math.max(min, Math.floor(value)));
}

function createId(): string {
  if (globalThis.crypto && "randomUUID" in globalThis.crypto) {
    return globalThis.crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
