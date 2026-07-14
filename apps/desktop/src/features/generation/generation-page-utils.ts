export function formatGenerationError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}
