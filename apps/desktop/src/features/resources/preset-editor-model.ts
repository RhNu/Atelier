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
  folderId: string | null;
  identifier: string;
  aliases: string[];
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
    folderId: null,
    identifier: "",
    aliases: [],
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
    folderId: preset.folder_id,
    identifier: preset.identifier,
    aliases: [...preset.aliases],
    kind: preset.kind,
    name: preset.display_name,
    category: parentPath(preset.path),
    description: preset.description ?? "",
    order: 0,
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
    path: [draft.category.trim(), draft.identifier || toIdentifier(draft.name)]
      .filter(Boolean)
      .join("/"),
    folder_id: draft.folderId,
    display_name: draft.name.trim(),
    aliases: draft.aliases,
    description: nullableText(draft.description),
    prompt_behavior: promptBehaviorToDto(draft.prompt),
    uc_behavior: promptBehaviorToDto(draft.uc),
    quality_override: kind === "main" && draft.qualityOverride ? draft.qualityOverride : null,
    uc_preset_override: kind === "main" ? nullableText(draft.ucPresetOverride) : null,
    preview: draft.preview,
    models: draft.models,
  };
}

function parentPath(path: string): string {
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
}

function toIdentifier(value: string): string {
  const normalized = value
    .trim()
    .normalize("NFC")
    .replace(/[^\p{L}\p{N}_-]+/gu, "_");
  return /^[\p{L}_]/u.test(normalized) ? normalized : `_${normalized || "preset"}`;
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
