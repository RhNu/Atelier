import type {
  GalleryItemDto,
  GallerySafetyLabelDto,
  GallerySafetyOverrideDto,
  GallerySourceKindDto,
  ResourceRefDto,
  VisualAssetDto,
} from "../../types";

export const PAGE_LIMIT = 24;

export type SafetyFilter = "all" | GallerySafetyLabelDto;
export type SourceFilter = "all" | GallerySourceKindDto;

export const sourceOptions = [
  { value: "all", label: "All sources" },
  { value: "generation", label: "Generation" },
  { value: "director", label: "Director" },
  { value: "import", label: "Import" },
] as const;

export const artifactOptions = [
  { value: "all", label: "All artifact kinds" },
  { value: "generated_image", label: "Generated images" },
  { value: "director_result", label: "Director results" },
] as const;

export const safetyFilterOptions = [
  { value: "all", label: "Safe + sensitive" },
  { value: "safe", label: "Safe" },
  { value: "sensitive", label: "Sensitive" },
  { value: "hidden", label: "Hidden" },
] as const;

export const overrideOptions = [
  { value: "", label: "Clear override" },
  { value: "safe", label: "Safe" },
  { value: "sensitive", label: "Sensitive" },
  { value: "hidden", label: "Hidden" },
] as const;

const INVALID_FILE_NAME_CHARACTERS = /[<>:"/\\|?*]+/g;

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function formatTimestamp(value: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function formatScore(label: string, value: number | null): string | null {
  if (value === null) {
    return null;
  }

  return `${label} ${value.toFixed(2)}`;
}

export function effectiveSafetyLabel(item: GalleryItemDto): GallerySafetyLabelDto | "unknown" {
  return item.safety?.effective_label ?? item.manual_safety_override ?? "unknown";
}

export function matchesSafetyFilter(item: GalleryItemDto, filter: SafetyFilter): boolean {
  if (filter === "all") {
    return item.manual_safety_override !== "hidden" && effectiveSafetyLabel(item) !== "hidden";
  }

  return effectiveSafetyLabel(item) === filter;
}

export function preferredThumbnailResource(item: GalleryItemDto): ResourceRefDto {
  return (
    item.assets.find((asset) => asset.role === "thumbnail")?.resource ??
    item.assets.find((asset) => asset.role === "primary")?.resource ??
    item.primary_resource
  );
}

export function preferredPreviewResource(item: GalleryItemDto): ResourceRefDto {
  return (
    item.assets.find((asset) => asset.role === "preview")?.resource ??
    item.assets.find((asset) => asset.role === "primary")?.resource ??
    item.primary_resource
  );
}

export function preferredExportAsset(item: GalleryItemDto): VisualAssetDto {
  return (
    item.assets.find((asset) => asset.role === "original") ??
    item.assets.find((asset) => asset.role === "primary") ??
    item.assets.find((asset) => asset.role === "preview") ??
    item.assets[0] ?? {
      role: "primary",
      resource: item.primary_resource,
      variant_kind: null,
    }
  );
}

export function suggestedGalleryExportFileName(itemId: string, role: string): string {
  const name = `${itemId}-${role}`
    .replace(INVALID_FILE_NAME_CHARACTERS, "-")
    .replace(/-+/g, "-")
    .replace(/[. ]+$/g, "")
    .trim();
  return name || "gallery-image";
}

export function parseSourceFilter(value: string): SourceFilter {
  switch (value) {
    case "generation":
    case "director":
    case "import":
      return value;
    default:
      return "all";
  }
}

export function parseSafetyFilter(value: string): SafetyFilter {
  switch (value) {
    case "safe":
    case "sensitive":
    case "hidden":
      return value;
    default:
      return "all";
  }
}

export function parseSafetyOverride(value: string): GallerySafetyOverrideDto | null {
  switch (value) {
    case "safe":
    case "sensitive":
    case "hidden":
      return value;
    default:
      return null;
  }
}
