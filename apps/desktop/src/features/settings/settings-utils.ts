import type { GlobalSettingsDto, WorkspaceSettingsDto } from "@/types";

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function createApiKeyId(): string {
  if (globalThis.crypto && "randomUUID" in globalThis.crypto) {
    return globalThis.crypto.randomUUID();
  }

  return `api-key-${Date.now()}`;
}

export function parseNumberInput(value: string): number {
  if (value.trim() === "") {
    return 0;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function isPositiveInteger(value: number): boolean {
  return Number.isInteger(value) && value > 0;
}

export function cloneSettings(settings: WorkspaceSettingsDto): WorkspaceSettingsDto {
  return {
    generation: {
      ...settings.generation,
      size: { ...settings.generation.size },
    },
    image_variants: { ...settings.image_variants },
  };
}

export function cloneGlobalSettings(settings: GlobalSettingsDto): GlobalSettingsDto {
  return {
    last_workspace: settings.last_workspace,
    frontend: {
      language: settings.frontend.language,
      developer_mode: settings.frontend.developer_mode,
      convert_full_width_punctuation: settings.frontend.convert_full_width_punctuation,
      gallery: { ...settings.frontend.gallery },
    },
    safety: { ...settings.safety },
  };
}
