import type {
  PromptChunkDto,
  PromptPresetDto,
  PromptPresetKindDto,
  UpsertPromptChunkRequestDto,
  UpsertPromptPresetRequestDto,
} from "../../types";

export type ResourceTab = "chunks" | "main-presets" | "character-presets" | "vibe";
export type ChunkDraft = UpsertPromptChunkRequestDto;
export type PresetDraft = UpsertPromptPresetRequestDto;

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function blankChunkDraft(): ChunkDraft {
  return {
    chunk_id: null,
    key: "",
    content: "",
    category: null,
    description: null,
    preview: null,
  };
}

export function chunkToDraft(chunk: PromptChunkDto): ChunkDraft {
  return {
    chunk_id: chunk.chunk_id,
    key: chunk.key,
    content: chunk.content,
    category: chunk.category,
    description: chunk.description,
    preview: chunk.preview,
  };
}

export function normalizeChunkDraft(draft: ChunkDraft): ChunkDraft {
  return {
    ...draft,
    key: draft.key.trim(),
    category: nullableText(draft.category ?? ""),
    description: nullableText(draft.description ?? ""),
  };
}

export function blankPresetDraft(kind: PromptPresetKindDto): PresetDraft {
  return {
    preset_id: null,
    kind,
    name: "",
    category: null,
    description: null,
    order: 0,
    enabled: true,
    before: "",
    after: "",
    replace: "",
    uc_before: "",
    uc_after: "",
    uc_replace: "",
    quality_override: null,
    uc_preset_override: null,
    preview: null,
  };
}

export function presetToDraft(preset: PromptPresetDto): PresetDraft {
  return {
    preset_id: preset.preset_id,
    kind: preset.kind,
    name: preset.name,
    category: preset.category,
    description: preset.description,
    order: preset.order,
    enabled: preset.enabled,
    before: preset.before,
    after: preset.after,
    replace: preset.replace,
    uc_before: preset.uc_before,
    uc_after: preset.uc_after,
    uc_replace: preset.uc_replace,
    quality_override: preset.quality_override,
    uc_preset_override: preset.uc_preset_override,
    preview: preset.preview,
  };
}

export function normalizePresetDraft(draft: PresetDraft, kind: PromptPresetKindDto): PresetDraft {
  return {
    ...draft,
    kind,
    name: draft.name.trim(),
    category: nullableText(draft.category ?? ""),
    description: nullableText(draft.description ?? ""),
    quality_override: kind === "main" ? nullableText(draft.quality_override ?? "") : null,
    uc_preset_override: kind === "main" ? nullableText(draft.uc_preset_override ?? "") : null,
  };
}

export function presetPreviewSource(draft: PresetDraft): string {
  return [
    draft.before,
    draft.replace,
    draft.after,
    draft.uc_before,
    draft.uc_replace,
    draft.uc_after,
  ]
    .filter((part) => part.trim().length > 0)
    .join("\n");
}

export function nullableText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function matchesSearch(search: string, ...values: Array<string | null>): boolean {
  const needle = search.trim().toLowerCase();
  if (!needle) {
    return true;
  }
  return values.some((value) => value?.toLowerCase().includes(needle));
}

export function parseTab(value: string): ResourceTab {
  switch (value) {
    case "main-presets":
    case "character-presets":
    case "vibe":
      return value;
    default:
      return "chunks";
  }
}

export function tabSummary(tab: ResourceTab): string {
  switch (tab) {
    case "chunks":
      return "Reusable @chunk(...) prompt fragments";
    case "main-presets":
      return "Global prompt presets and generation overrides";
    case "character-presets":
      return "Character prompt presets without generation overrides";
    case "vibe":
      return "NovelAI Vibe documents and encodings";
  }
}
