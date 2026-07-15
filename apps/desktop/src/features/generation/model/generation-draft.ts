/* eslint-disable max-lines */
import type {
  CharacterReferenceTypeDto,
  CharacterDto,
  GenerateImageRequestDto,
  GenerationPlanContextDto,
  GenerationEstimateRequestDto,
  GenerationDraftDto,
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
  presetId: string | null;
  prompt: string;
  negativePrompt: string;
  enabled: boolean;
  position: { x: number; y: number };
};

export type GenerationDraft = {
  mainPresetId: string | null;
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
    mainPresetId: null,
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
    vibe: { strength: 1, slots: [] },
    preciseReferences: [],
    characters: [],
    characterPositionMode: "global",
  };
}

export function generationDraftFromDto(value: GenerationDraftDto): GenerationDraft {
  return {
    mainPresetId: value.main_preset_id ?? null,
    prompt: value.prompt,
    negativePrompt: value.negative_prompt,
    model: value.model,
    size: { ...value.size },
    quality: value.quality,
    ucPreset: value.uc_preset,
    steps: value.steps,
    scale: value.scale,
    sampler: value.sampler,
    noiseSchedule: value.noise_schedule,
    seedMode: value.seed_mode,
    seed: value.seed,
    nSamples: value.n_samples,
    requestCount: value.request_count,
    cfgRescale: value.cfg_rescale,
    varietyBoost: value.variety_boost,
    imageFormat: value.image_format ?? null,
    strictMode: value.strict_mode,
    streamEnabled: value.stream_enabled,
    i2i: value.i2i
      ? {
          image: value.i2i.image,
          mask: value.i2i.mask ?? null,
          strength: value.i2i.strength,
          noise: value.i2i.noise,
        }
      : null,
    vibe: {
      strength: value.vibe.strength,
      slots: value.vibe.slots.map((slot) => ({
        id: slot.id,
        encoding: slot.encoding,
        vibeId: slot.vibe_id ?? null,
        informationExtracted: slot.information_extracted,
        strength: slot.strength,
        displayName: slot.display_name,
        sourceImage: slot.source_image ?? null,
        sourceSha256: slot.source_sha256 ?? null,
      })),
    },
    preciseReferences: value.precise_references.map((reference) => ({
      id: reference.id,
      image: reference.image,
      referenceType: reference.reference_type,
      fidelity: reference.fidelity,
      strength: reference.strength,
      displayName: reference.display_name,
    })),
    characters: value.characters.map((character) => ({
      id: character.id,
      presetId: character.preset_id ?? null,
      prompt: character.prompt,
      negativePrompt: character.negative_prompt,
      enabled: character.enabled,
      position: { ...character.position },
    })),
    characterPositionMode: value.character_position_mode,
  };
}

export function generationDraftToDto(value: GenerationDraft): GenerationDraftDto {
  return {
    main_preset_id: value.mainPresetId,
    prompt: value.prompt,
    negative_prompt: value.negativePrompt,
    model: value.model,
    size: { ...value.size },
    quality: value.quality,
    uc_preset: value.ucPreset,
    steps: value.steps,
    scale: value.scale,
    sampler: value.sampler,
    noise_schedule: value.noiseSchedule,
    seed_mode: value.seedMode,
    seed: value.seed,
    n_samples: value.nSamples,
    request_count: value.requestCount,
    cfg_rescale: value.cfgRescale,
    variety_boost: value.varietyBoost,
    image_format: value.imageFormat,
    strict_mode: value.strictMode,
    stream_enabled: value.streamEnabled,
    i2i: value.i2i
      ? {
          image: value.i2i.image,
          mask: value.i2i.mask,
          strength: value.i2i.strength,
          noise: value.i2i.noise,
        }
      : null,
    vibe: {
      enabled: isVibeActive(value),
      strength: value.vibe.strength,
      slots: value.vibe.slots.map((slot) => ({
        id: slot.id,
        encoding: slot.encoding,
        vibe_id: slot.vibeId ?? null,
        information_extracted: slot.informationExtracted,
        strength: slot.strength,
        display_name: slot.displayName,
        source_image: slot.sourceImage,
        source_sha256: slot.sourceSha256,
      })),
    },
    precise_references: value.preciseReferences.map((reference) => ({
      id: reference.id,
      image: reference.image,
      reference_type: reference.referenceType,
      fidelity: reference.fidelity,
      strength: reference.strength,
      display_name: reference.displayName,
    })),
    characters: value.characters.map((character) => ({
      id: character.id,
      preset_id: character.presetId,
      prompt: character.prompt,
      negative_prompt: character.negativePrompt,
      enabled: character.enabled,
      position: { ...character.position },
    })),
    character_position_mode: value.characterPositionMode,
  };
}

export function resetGenerationParameters(
  draft: GenerationDraft,
  settings: WorkspaceSettingsDto,
): GenerationDraft {
  const reset = createGenerationDraft(settings);
  return {
    ...draft,
    model: reset.model,
    size: reset.size,
    quality: reset.quality,
    ucPreset: reset.ucPreset,
    steps: reset.steps,
    scale: reset.scale,
    sampler: reset.sampler,
    noiseSchedule: reset.noiseSchedule,
    seedMode: reset.seedMode,
    seed: reset.seed,
    nSamples: reset.nSamples,
    requestCount: reset.requestCount,
    cfgRescale: reset.cfgRescale,
    varietyBoost: reset.varietyBoost,
    imageFormat: reset.imageFormat,
    strictMode: reset.strictMode,
    streamEnabled: reset.streamEnabled,
  };
}

export function canSubmitGenerationDraft(draft: GenerationDraft): boolean {
  return draft.prompt.trim().length > 0 || Boolean(draft.mainPresetId);
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
    mainPresetId: draft.mainPresetId,
    characterPresetIds: draft.characters.map((character) => character.presetId ?? null),
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
    main_preset_id: draft.mainPresetId,
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
    use_coords:
      characters && characters.length >= 2 ? draft.characterPositionMode === "manual" : null,
    image_format: draft.imageFormat,
  };
}

function buildCharacters(draft: GenerationDraft): CharacterDto[] | null {
  const eligibleCharacters = draft.characters.filter(
    (character) =>
      character.enabled && (character.prompt.trim().length > 0 || Boolean(character.presetId)),
  );
  const useManualPositions =
    eligibleCharacters.length >= 2 && draft.characterPositionMode === "manual";
  const characters = eligibleCharacters.map((character) => ({
    preset_id: character.presetId,
    prompt: character.prompt,
    negative_prompt: normalizeOptionalText(character.negativePrompt),
    position: useManualPositions ? { ...character.position } : { x: 0.5, y: 0.5 },
    enabled: true,
  }));
  return characters.length ? characters : null;
}

function buildPendingVibeEncodeCount(draft: GenerationDraft): number {
  return isVibeActive(draft) ? draft.vibe.slots.filter((slot) => !slot.encoding).length : 0;
}

function buildControlNet(draft: GenerationDraft): GenerateImageRequestDto["controlnet"] {
  const images = isVibeActive(draft)
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

export function isVibeActive(draft: GenerationDraft): boolean {
  return draft.preciseReferences.length === 0 && draft.vibe.slots.length > 0;
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
