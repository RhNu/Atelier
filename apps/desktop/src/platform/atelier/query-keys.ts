import type { GalleryQueryDto, ResourceRefDto, RunHistoryQueryDto } from "../../types";

type GalleryQueryKeyInput = Pick<GalleryQueryDto, "limit" | "offset"> &
  Partial<Omit<GalleryQueryDto, "limit" | "offset">>;

function normalizeResourceRef(resource: ResourceRefDto): ResourceRefDto {
  return {
    id: resource.id,
    variant_id: resource.variant_id ?? null,
  };
}

export const queryKeys = {
  workspace: {
    root: () => ["workspace"] as const,
    status: () => ["workspace", "status"] as const,
  },
  account: {
    root: () => ["account"] as const,
    apiKeys: () => ["account", "api-keys"] as const,
    activeProbe: () => ["account", "active-probe"] as const,
  },
  settings: {
    root: () => ["settings"] as const,
    workspace: () => ["settings", "workspace"] as const,
  },
  generation: {
    root: () => ["generation"] as const,
    status: (jobId?: string | null) => ["generation", "status", jobId ?? null] as const,
  },
  history: {
    root: () => ["history"] as const,
    list: (query: RunHistoryQueryDto) => ["history", "list", query] as const,
  },
  gallery: {
    root: () => ["gallery"] as const,
    list: (query: GalleryQueryKeyInput) => ["gallery", "list", query] as const,
  },
  prompt: {
    root: () => ["prompt"] as const,
    chunks: () => ["prompt", "chunks"] as const,
    lexiconCatalog: () => ["prompt", "lexicon", "catalog"] as const,
    lexiconList: (query: unknown) => ["prompt", "lexicon", "list", query] as const,
    lexiconSearch: (query: unknown) => ["prompt", "lexicon", "search", query] as const,
  },
  resource: {
    root: () => ["resource"] as const,
    image: (resource: ResourceRefDto) =>
      ["resource", "image", normalizeResourceRef(resource)] as const,
  },
  vibe: {
    root: () => ["vibe"] as const,
  },
  director: {
    root: () => ["director"] as const,
  },
} as const;
