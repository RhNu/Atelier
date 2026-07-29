export type ResourceTab = "chunks" | "main-presets" | "character-presets" | "vibe";
export type ResourceViewMode = "list" | "grid";

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function nullableText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function categorySuggestions(values: ReadonlyArray<string | null>): string[] {
  return [...new Set(values.flatMap((value) => (value?.trim() ? [value.trim()] : [])))].sort(
    (left, right) => left.localeCompare(right),
  );
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
