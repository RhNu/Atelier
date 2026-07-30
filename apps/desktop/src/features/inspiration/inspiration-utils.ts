import type { DanbooruRatingDto, DanbooruTagDto } from "@/types";

const RATING_TOKENS =
  /(^|\s)-?rating:|(^|\s)is:(?:sfw|nsfw|general|sensitive|questionable|explicit)(?:\s|$)/i;

export function hasRatingMetatag(query: string): boolean {
  return RATING_TOKENS.test(query);
}

export function currentTagToken(query: string): string {
  return query.match(/(?:^|\s)([^\s]*)$/)?.[1] ?? "";
}

export function replaceCurrentToken(query: string, tag: string): string {
  const start = query.search(/[^\s]*$/);
  return `${query.slice(0, Math.max(0, start))}${tag} `;
}

export function appendSearchTag(query: string, tag: string): string {
  const tokens = query.split(/\s+/).filter(Boolean);
  if (!tokens.includes(tag)) tokens.push(tag);
  return tokens.join(" ");
}

export function selectedRatings(showAdult: boolean): DanbooruRatingDto[] {
  return showAdult
    ? ["general", "sensitive", "questionable", "explicit"]
    : ["general", "sensitive"];
}

export function shouldBlurRating(rating: DanbooruRatingDto, blurSensitive: boolean): boolean {
  return blurSensitive && rating !== "general";
}

export function formatPromptTags(tags: DanbooruTagDto[]): string {
  return tags.map((tag) => tag.canonical_name).join(", ");
}

export function formatQueryTags(tags: DanbooruTagDto[]): string {
  return tags.map((tag) => tag.canonical_name).join(" ");
}

const CATEGORY_ORDER = {
  artist: 0,
  copyright: 1,
  character: 2,
  general: 3,
  meta: 4,
} as const;

export function orderSelectedTags(tags: Iterable<DanbooruTagDto>): DanbooruTagDto[] {
  return Array.from(tags).toSorted(
    (left, right) => CATEGORY_ORDER[left.category] - CATEGORY_ORDER[right.category],
  );
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

export function formatError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
