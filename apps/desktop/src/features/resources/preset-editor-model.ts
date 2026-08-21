import type {
  PromptPresetBehaviorDto,
  PromptPresetDto,
  PromptPresetKindDto,
  ImageModelDto,
  QualityPresetDto,
  ResourceRefDto,
  UpsertPromptPresetRequestDto,
} from "@/types";

import { nullableText } from "./resource-model";

export type PromptBehaviorMode = PromptPresetBehaviorDto["mode"];

export type PromptBehaviorDraft = {
  mode: PromptBehaviorMode;
  before: string;
  after: string;
  replacement: string;
};

export type PresetEditorDraft = {
  presetId: string | null;
  kind: PromptPresetKindDto;
  name: string;
  category: string;
  description: string;
  order: number;
  prompt: PromptBehaviorDraft;
  uc: PromptBehaviorDraft;
  qualityOverride: QualityPresetDto | "";
  ucPresetOverride: string;
  preview: ResourceRefDto | null;
  models: ImageModelDto[];
};

export function blankPresetEditorDraft(
  kind: PromptPresetKindDto,
  model: ImageModelDto = "nai-diffusion-4-5-full",
): PresetEditorDraft {
  return {
    presetId: null,
    kind,
    name: "",
    category: "",
    description: "",
    order: 0,
    prompt: blankPromptBehavior(),
    uc: blankPromptBehavior(),
    qualityOverride: "",
    ucPresetOverride: "",
    preview: null,
    models: [model],
  };
}

export function presetToEditorDraft(preset: PromptPresetDto): PresetEditorDraft {
  return {
    presetId: preset.preset_id,
    kind: preset.kind,
    name: preset.name,
    category: preset.category ?? "",
    description: preset.description ?? "",
    order: preset.order,
    prompt: promptBehaviorToDraft(preset.prompt_behavior),
    uc: promptBehaviorToDraft(preset.uc_behavior),
    qualityOverride: preset.quality_override ?? "",
    ucPresetOverride: preset.uc_preset_override ?? "",
    preview: preset.preview,
    models: [...preset.models],
  };
}

export function editorDraftToUpsertRequest(
  draft: PresetEditorDraft,
  kind: PromptPresetKindDto,
): UpsertPromptPresetRequestDto {
  return {
    preset_id: draft.presetId,
    kind,
    name: draft.name.trim(),
    category: nullableText(draft.category),
    description: nullableText(draft.description),
    order: draft.order,
    prompt_behavior: promptBehaviorToDto(draft.prompt),
    uc_behavior: promptBehaviorToDto(draft.uc),
    quality_override: kind === "main" && draft.qualityOverride ? draft.qualityOverride : null,
    uc_preset_override: kind === "main" ? nullableText(draft.ucPresetOverride) : null,
    preview: draft.preview,
    models: draft.models,
  };
}

export function presetPreviewSource(draft: PresetEditorDraft): string {
  return [activePromptText(draft.prompt), activePromptText(draft.uc)]
    .filter((part) => part.trim().length > 0)
    .join("\n");
}

export function presetSearchText(preset: PromptPresetDto): string {
  const behavior = preset.prompt_behavior;
  return behavior.mode === "replace" ? behavior.text : `${behavior.before} ${behavior.after}`;
}

function blankPromptBehavior(): PromptBehaviorDraft {
  return {
    mode: "surround",
    before: "",
    after: "",
    replacement: "",
  };
}

function promptBehaviorToDraft(behavior: PromptPresetBehaviorDto): PromptBehaviorDraft {
  return behavior.mode === "replace"
    ? {
        mode: "replace",
        before: "",
        after: "",
        replacement: behavior.text,
      }
    : {
        mode: "surround",
        before: behavior.before,
        after: behavior.after,
        replacement: "",
      };
}

function promptBehaviorToDto(behavior: PromptBehaviorDraft): PromptPresetBehaviorDto {
  return behavior.mode === "replace"
    ? { mode: "replace", text: behavior.replacement }
    : { mode: "surround", before: behavior.before, after: behavior.after };
}

function activePromptText(behavior: PromptBehaviorDraft): string {
  return behavior.mode === "replace"
    ? behavior.replacement
    : [behavior.before, behavior.after].filter(Boolean).join("\n");
}
