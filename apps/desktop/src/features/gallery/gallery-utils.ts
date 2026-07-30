import { i18n } from "@/i18n";
import type {
  GalleryItemDto,
  ImageExportFormatDto,
  GallerySafetyLabelDto,
  GallerySafetyOverrideDto,
  GallerySourceKindDto,
  ResourceRefDto,
  VisualAssetDto,
} from "@/types";

import { generationModelDisplayNames } from "../generation/model/generation-options";

export const PAGE_LIMIT = 24;

export type SafetyFilter = "all" | GallerySafetyLabelDto;
export type SourceFilter = "all" | GallerySourceKindDto;

export const sourceOptions = [
  { value: "all", labelKey: "allSources" },
  { value: "generation", labelKey: "generation" },
  { value: "director", labelKey: "director" },
] as const;

export const artifactOptions = [
  { value: "all", labelKey: "allArtifacts" },
  { value: "generated_image", labelKey: "generatedImages" },
  { value: "director_result", labelKey: "directorResults" },
] as const;

export const safetyFilterOptions = [
  { value: "all", labelKey: "safeSensitive" },
  { value: "safe", labelKey: "safe" },
  { value: "sensitive", labelKey: "sensitive" },
  { value: "hidden", labelKey: "hidden" },
] as const;

export const overrideOptions = [
  { value: "", labelKey: "clearOverride" },
  { value: "safe", labelKey: "safe" },
  { value: "sensitive", labelKey: "sensitive" },
  { value: "hidden", labelKey: "hidden" },
] as const;

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function formatTimestamp(value: number): string {
  return new Intl.DateTimeFormat(i18n.resolvedLanguage, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function effectiveSafetyLabel(item: GalleryItemDto): GallerySafetyLabelDto | "unknown" {
  return item.safety.effective_label ?? item.manual_safety_override ?? "unknown";
}

export function displayGalleryModelName(modelName: string | null): string | null {
  if (!modelName) {
    return null;
  }

  const displayNames: Record<string, string> = generationModelDisplayNames;
  return displayNames[modelName] ?? modelName;
}

export function displayGalleryArtifactKind(
  artifactKind: string,
  translate: (key: "generatedImages" | "directorResults") => string,
): string {
  switch (artifactKind) {
    case "generated_image":
      return translate("generatedImages");
    case "director_result":
      return translate("directorResults");
    default:
      return artifactKind;
  }
}

export function displayGallerySource(
  sourceKind: GallerySourceKindDto,
  translate: (key: "generation" | "director") => string,
): string {
  return translate(sourceKind === "generation" ? "generation" : "director");
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

export function suggestedGalleryExportFileName(
  indexedAtMs: number,
  format: ImageExportFormatDto,
): string {
  const date = new Date(indexedAtMs);
  if (Number.isNaN(date.getTime())) {
    return `image-${format === "jpeg" ? "jpg" : "png"}`;
  }
  const twoDigits = (value: number) => String(value).padStart(2, "0");
  return [
    "image-",
    twoDigits(date.getUTCFullYear() % 100),
    twoDigits(date.getUTCMonth() + 1),
    twoDigits(date.getUTCDate()),
    "-",
    twoDigits(date.getUTCHours()),
    twoDigits(date.getUTCMinutes()),
    twoDigits(date.getUTCSeconds()),
  ].join("");
}

export function parseSourceFilter(value: string): SourceFilter {
  switch (value) {
    case "generation":
    case "director":
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
