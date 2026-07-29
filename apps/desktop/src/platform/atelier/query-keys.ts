import type {
  GalleryQueryDto,
  GenerationHistoryQueryDto,
  ListPromptPresetsRequestDto,
  ListVibeDocumentsRequestDto,
  LexiconSearchRequestDto,
  ResourceRefDto,
  RunHistoryQueryDto,
} from "@/types";

type GalleryQueryKeyInput = Pick<GalleryQueryDto, "limit" | "offset"> &
  Partial<Omit<GalleryQueryDto, "limit" | "offset">>;

function normalizeResourceRef(resource: ResourceRefDto): ResourceRefDto {
  return {
    id: resource.id,
    variant_id: resource.variant_id ?? null,
  };
}

export const queryKeys = {
  app: {
    root: () => ["app"] as const,
    bootstrap: () => ["app", "bootstrap"] as const,
    globalSettings: () => ["app", "settings"] as const,
  },
  workspace: {
    root: () => ["workspace"] as const,
    status: () => ["app", "workspace-status"] as const,
  },
  account: {
    root: () => ["workspace", "account"] as const,
    apiKeys: () => ["workspace", "account", "api-keys"] as const,
    activeSummary: () => ["workspace", "account", "active-summary"] as const,
  },
  settings: {
    root: () => ["workspace", "settings"] as const,
    workspace: () => ["workspace", "settings"] as const,
  },
  generation: {
    root: () => ["workspace", "generation"] as const,
    draft: () => ["workspace", "generation", "draft"] as const,
    status: (jobId?: string | null) =>
      ["workspace", "generation", "status", jobId ?? null] as const,
    estimate: (request: unknown) => ["workspace", "generation", "estimate", request] as const,
  },
  history: {
    root: () => ["workspace", "history"] as const,
    list: (query: RunHistoryQueryDto) => ["workspace", "history", "list", query] as const,
    generationBatches: (query: GenerationHistoryQueryDto) =>
      ["workspace", "history", "generation-batches", query] as const,
    generationBatch: (batchId: string | null) =>
      ["workspace", "history", "generation-batch", batchId] as const,
  },
  gallery: {
    root: () => ["workspace", "gallery"] as const,
    list: (query: GalleryQueryKeyInput) => ["workspace", "gallery", "list", query] as const,
    detail: (itemId: string | null) => ["workspace", "gallery", "detail", itemId] as const,
  },
  prompt: {
    root: () => ["workspace", "prompt"] as const,
    chunks: (query?: unknown) =>
      query === undefined
        ? (["workspace", "prompt", "chunks"] as const)
        : (["workspace", "prompt", "chunks", query] as const),
    presets: (query: ListPromptPresetsRequestDto) =>
      ["workspace", "prompt", "presets", query] as const,
  },
  lexicon: {
    root: () => ["app", "lexicon"] as const,
    bootstrap: () => ["app", "lexicon", "bootstrap"] as const,
    completion: (query: string, limit: number) =>
      ["app", "lexicon", "completion", query, limit] as const,
    search: (query: LexiconSearchRequestDto) => ["app", "lexicon", "search", query] as const,
    entity: (entityId: number | null) => ["app", "lexicon", "entity", entityId] as const,
  },
  resource: {
    root: () => ["workspace", "resource"] as const,
    image: (resource: ResourceRefDto) =>
      ["workspace", "resource", "image", normalizeResourceRef(resource)] as const,
  },
  vibe: {
    root: () => ["workspace", "vibe"] as const,
    list: (query: ListVibeDocumentsRequestDto) => ["workspace", "vibe", "list", query] as const,
    get: (vibeId: string) => ["workspace", "vibe", "get", vibeId] as const,
  },
  director: {
    root: () => ["workspace", "director"] as const,
  },
} as const;
