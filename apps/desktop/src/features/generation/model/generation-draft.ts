/* eslint-disable max-lines */
import type {
  CharacterDto,
  CharacterReferenceTypeDto,
  GenerateImageRequestDto,
  GenerationDraftDto,
  GenerationEstimateRequestDto,
  GenerationPlanContextDto,
  GenerationWorkRequestDto,
  ImageFormatDto,
  ImageInputDto,
  ImageModelDto,
  ImageSizeDto,
  ModelCapabilitiesDto,
  NoiseScheduleDto,
  QualityPresetDto,
  ResourceRefDto,
  SamplerDto,
  SubscriptionSummaryDto,
  SubmitGenerationBatchRequestDto,
  UcPresetDto,
  WorkspaceSettingsDto,
} from "@/types";

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
  model: ImageModelDto;
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
export type GenerationPromptState = {
  model: ImageModelDto;
  mainPresetId: string | null;
  prompt: string;
  negativePrompt: string;
  characters: GenerationCharacterDraft[];
  characterPositionMode: GenerationCharacterPositionMode;
};
export type GenerationDraft = {
  mainPresetId: string | null;
  prompt: string;
  negativePrompt: string;
  model: ImageModelDto;
  promptStates: GenerationPromptState[];
  size: ImageSizeDto;
  quality: QualityPresetDto;
  transparentBackground: boolean;
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
  vibe: { strength: number; slots: GenerationVibeSlotDraft[] };
  preciseReferences: GenerationPreciseReferenceDraft[];
  characters: GenerationCharacterDraft[];
  characterPositionMode: GenerationCharacterPositionMode;
};
export type GenerationRunIds = { batchId: string; jobIds: string[] };
export type GenerationPlanOptions = {
  subscription?: SubscriptionSummaryDto | null;
  capabilities?: ModelCapabilitiesDto;
};

function activePromptState(draft: GenerationDraft): GenerationPromptState {
  return {
    model: draft.model,
    mainPresetId: draft.mainPresetId,
    prompt: draft.prompt,
    negativePrompt: draft.negativePrompt,
    characters: draft.characters.map(copyCharacter),
    characterPositionMode: draft.characterPositionMode,
  };
}

function mergeActivePromptState(draft: GenerationDraft): GenerationPromptState[] {
  return [
    ...draft.promptStates.filter((state) => state.model !== draft.model),
    activePromptState(draft),
  ];
}

export function switchGenerationModel(
  draft: GenerationDraft,
  model: ImageModelDto,
  capabilities: ModelCapabilitiesDto,
): GenerationDraft {
  if (model === draft.model) return draft;
  const states = mergeActivePromptState(draft);
  const restored = states.find((state) => state.model === model) ?? emptyPromptState(model);
  return {
    ...draft,
    model,
    promptStates: states,
    scale: capabilities.default_scale,
    mainPresetId: restored.mainPresetId,
    prompt: restored.prompt,
    negativePrompt: restored.negativePrompt,
    characters: restored.characters.map(copyCharacter),
    characterPositionMode: restored.characterPositionMode,
  };
}

export function createGenerationDraft(settings: WorkspaceSettingsDto): GenerationDraft {
  const defaults = settings.generation;
  return {
    ...emptyPromptState(defaults.model),
    promptStates: [],
    size: { ...defaults.size },
    quality: defaults.quality,
    transparentBackground: defaults.transparent_background,
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
  };
}

export function generationDraftFromDto(value: GenerationDraftDto): GenerationDraft {
  const promptStates = value.prompt_states.map((state) => ({
    model: state.model,
    mainPresetId: state.main_preset_id,
    prompt: state.prompt,
    negativePrompt: state.negative_prompt,
    characters: state.characters.map(fromCharacterDto),
    characterPositionMode: state.character_position_mode,
  }));
  const active =
    promptStates.find((state) => state.model === value.model) ?? emptyPromptState(value.model);
  return {
    ...active,
    characters: active.characters.map(copyCharacter),
    promptStates,
    size: { ...value.size },
    quality: value.quality,
    transparentBackground: value.transparent_background,
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
    imageFormat: value.image_format,
    strictMode: value.strict_mode,
    streamEnabled: value.stream_enabled,
    i2i: value.i2i
      ? {
          image: value.i2i.image,
          mask: value.i2i.mask,
          strength: value.i2i.strength,
          noise: value.i2i.noise,
        }
      : null,
    vibe: {
      strength: value.vibe.strength,
      slots: value.vibe.slots.map((slot) => ({
        id: slot.id,
        encoding: slot.encoding,
        vibeId: slot.vibe_id,
        informationExtracted: slot.information_extracted,
        strength: slot.strength,
        displayName: slot.display_name,
        sourceImage: slot.source_image,
        sourceSha256: slot.source_sha256,
        model: slot.model,
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
  };
}

export function generationDraftToDto(value: GenerationDraft): GenerationDraftDto {
  return {
    model: value.model,
    prompt_states: mergeActivePromptState(value).map((state) => ({
      model: state.model,
      main_preset_id: state.mainPresetId,
      prompt: state.prompt,
      negative_prompt: state.negativePrompt,
      characters: state.characters.map(toCharacterDto),
      character_position_mode: state.characterPositionMode,
    })),
    size: { ...value.size },
    quality: value.quality,
    transparent_background: value.transparentBackground,
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
        model: slot.model,
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
  };
}

export function resetGenerationParameters(
  draft: GenerationDraft,
  settings: WorkspaceSettingsDto,
): GenerationDraft {
  const reset = createGenerationDraft(settings);
  return {
    ...draft,
    size: reset.size,
    quality: reset.quality,
    transparentBackground: reset.transparentBackground,
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
    work: buildGenerationWorkRequest(draft, options.capabilities),
  }));
  return {
    batch_id: ids.batchId,
    jobs,
    context: buildGenerationPlanContext(
      draft,
      jobs.length,
      options.subscription,
      options.capabilities,
    ),
  };
}

export function buildGenerationEstimateRequest(
  draft: GenerationDraft,
  options: GenerationPlanOptions = {},
): GenerationEstimateRequestDto {
  return {
    request: buildBaseGenerateRequest(draft, options.capabilities),
    context: buildGenerationPlanContext(
      draft,
      draft.requestCount,
      options.subscription,
      options.capabilities,
    ),
  };
}

export function buildGenerationEstimateCacheKey(
  draft: GenerationDraft,
  options: GenerationPlanOptions = {},
) {
  const preciseReferences = buildPreciseReferences(draft, options.capabilities);
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
    vibeSlotCount: preciseReferences
      ? 0
      : (buildVibeTransfer(draft, options.capabilities)?.references.length ?? 0),
    pendingVibeEncodeCount: buildPendingVibeEncodeCount(draft, options.capabilities),
    subscriptionTier: options.subscription?.tier ?? 0,
    subscriptionActive: options.subscription?.subscription_active ?? false,
    v5UsageIsNegative: options.subscription?.v5_usage?.is_negative ?? false,
  };
}

function buildGenerationWorkRequest(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): GenerationWorkRequestDto {
  const base = buildBaseGenerateRequest(draft, capabilities);
  return draft.streamEnabled && capabilities?.supports_streaming !== false
    ? { kind: "stream", request: { base, stream: "sse" } }
    : { kind: "image", request: base };
}

function buildGenerationPlanContext(
  draft: GenerationDraft,
  requestCount: number,
  subscription?: SubscriptionSummaryDto | null,
  capabilities?: ModelCapabilitiesDto,
): GenerationPlanContextDto {
  return {
    request_count: clampInteger(requestCount, 1, 8),
    pending_vibe_encode_count: buildPendingVibeEncodeCount(draft, capabilities),
    tier: subscription?.tier ?? 0,
    subscription_active: subscription?.subscription_active ?? false,
    v5_usage_is_negative: subscription?.v5_usage?.is_negative ?? false,
  };
}

function buildBaseGenerateRequest(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): GenerateImageRequestDto {
  const preciseReferences = buildPreciseReferences(draft, capabilities);
  const characters = buildCharacters(draft, capabilities);
  const quality =
    draft.quality === "light" && capabilities && !capabilities.supports_light_quality_preset
      ? "standard"
      : draft.quality;
  return {
    main_preset_id: draft.mainPresetId,
    prompt: draft.prompt,
    model: draft.model,
    size: { ...draft.size },
    negative_prompt: normalizeOptionalText(draft.negativePrompt),
    quality,
    transparent_background: capabilities?.supports_transparent_background
      ? draft.transparentBackground
      : false,
    uc_preset: draft.ucPreset,
    steps: draft.steps,
    scale: draft.scale,
    sampler: draft.sampler,
    noise_schedule: draft.noiseSchedule,
    seed: draft.seedMode === "fixed" ? draft.seed : 0,
    n_samples: draft.nSamples,
    cfg_rescale: draft.cfgRescale,
    variety_boost: capabilities?.supports_variety_boost === false ? false : draft.varietyBoost,
    strict_mode: draft.strictMode,
    img2img: draft.i2i
      ? {
          image: resourceImageInput(draft.i2i.image),
          strength: draft.i2i.strength,
          noise: draft.i2i.noise,
          mask: draft.i2i.mask ? resourceImageInput(draft.i2i.mask) : null,
        }
      : null,
    vibe_transfer: preciseReferences ? null : buildVibeTransfer(draft, capabilities),
    character_references: preciseReferences,
    characters,
    use_coords: characters?.length
      ? draft.characterPositionMode === "manual" &&
        characters.length >= (capabilities?.can_position_one_character ? 1 : 2)
      : null,
    image_format: draft.imageFormat,
  };
}

function buildCharacters(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): CharacterDto[] | null {
  if (capabilities?.max_characters === 0) return null;
  const eligible = draft.characters.filter(
    (character) =>
      character.enabled && (character.prompt.trim().length > 0 || Boolean(character.presetId)),
  );
  const limited = capabilities ? eligible.slice(0, capabilities.max_characters) : eligible;
  const manual =
    limited.length >= (capabilities?.can_position_one_character ? 1 : 2) &&
    draft.characterPositionMode === "manual";
  const characters = limited.map((character) => ({
    preset_id: character.presetId,
    prompt: character.prompt,
    negative_prompt: normalizeOptionalText(character.negativePrompt),
    position: manual ? { ...character.position } : { x: 0.5, y: 0.5 },
    enabled: true,
  }));
  return characters.length ? characters : null;
}

function currentVibeSlots(draft: GenerationDraft) {
  return draft.vibe.slots.filter((slot) => slot.model === draft.model);
}

function buildPendingVibeEncodeCount(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): number {
  return isVibeActive(draft, capabilities)
    ? currentVibeSlots(draft).filter((slot) => !slot.encoding).length
    : 0;
}

function buildVibeTransfer(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): GenerateImageRequestDto["vibe_transfer"] {
  if (capabilities?.supports_vibe_transfer === false) return null;
  const references = isVibeActive(draft, capabilities)
    ? currentVibeSlots(draft)
        .filter((slot) => Boolean(slot.encoding))
        .map((slot) => ({ encoding: slot.encoding, strength: slot.strength }))
    : [];
  return references.length ? { references, strength: draft.vibe.strength } : null;
}

export function isVibeActive(draft: GenerationDraft, capabilities?: ModelCapabilitiesDto): boolean {
  return (
    capabilities?.supports_vibe_transfer !== false &&
    draft.preciseReferences.length === 0 &&
    currentVibeSlots(draft).length > 0
  );
}

function buildPreciseReferences(
  draft: GenerationDraft,
  capabilities?: ModelCapabilitiesDto,
): GenerateImageRequestDto["character_references"] {
  if (
    capabilities?.supports_character_reference === false ||
    draft.preciseReferences.length === 0
  ) {
    return null;
  }
  return draft.preciseReferences.map((reference) => ({
    image: resourceImageInput(reference.image),
    reference_type: reference.referenceType,
    fidelity: reference.fidelity,
    strength: reference.strength,
  }));
}

function emptyPromptState(model: ImageModelDto): GenerationPromptState {
  return {
    model,
    mainPresetId: null,
    prompt: "",
    negativePrompt: "",
    characters: [],
    characterPositionMode: "global",
  };
}

function copyCharacter(character: GenerationCharacterDraft): GenerationCharacterDraft {
  return { ...character, position: { ...character.position } };
}

function fromCharacterDto(
  character: GenerationDraftDto["prompt_states"][number]["characters"][number],
): GenerationCharacterDraft {
  return {
    id: character.id,
    presetId: character.preset_id,
    prompt: character.prompt,
    negativePrompt: character.negative_prompt,
    enabled: character.enabled,
    position: { ...character.position },
  };
}

function toCharacterDto(character: GenerationCharacterDraft) {
  return {
    id: character.id,
    preset_id: character.presetId,
    prompt: character.prompt,
    negative_prompt: character.negativePrompt,
    enabled: character.enabled,
    position: { ...character.position },
  };
}

function resourceImageInput(resource: ResourceRefDto): ImageInputDto {
  return { kind: "resource_ref", resource };
}
function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
function clampInteger(value: number, min: number, max: number): number {
  return Number.isFinite(value) ? Math.min(max, Math.max(min, Math.floor(value))) : min;
}
function createId(): string {
  return globalThis.crypto && "randomUUID" in globalThis.crypto
    ? globalThis.crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
