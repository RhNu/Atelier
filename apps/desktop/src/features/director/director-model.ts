import type {
  DirectorToolDto,
  GallerySafetyOverrideDto,
  ImageInputDto,
  ResourceRefDto,
} from "@/types";

export type DirectorInput =
  | { kind: "resource"; resource: ResourceRefDto; label: string }
  | { kind: "inline"; imageBase64: string; src: string; label: string };

export const DIRECTOR_TOOLS: ReadonlyArray<{
  value: DirectorToolDto;
}> = [
  { value: "lineart" },
  { value: "sketch" },
  { value: "bg_removal" },
  { value: "declutter" },
  { value: "colorize" },
  { value: "emotion" },
];

export function buildDirectorRunRequest(
  input: DirectorInput,
  tool: DirectorToolDto,
  prompt: string,
  defry: number,
) {
  const image: ImageInputDto =
    input.kind === "resource"
      ? { kind: "resource_ref", resource: input.resource }
      : { kind: "inline_base64", image_base64: input.imageBase64 };
  const supportsPrompt = tool === "colorize" || tool === "emotion";
  return {
    run_id: `director-${createId()}`,
    tool,
    image,
    prompt: supportsPrompt && prompt.trim().length > 0 ? prompt.trim() : null,
    defry: supportsPrompt ? clampDefry(defry) : null,
    strict_mode: true,
  };
}

export function parseDirectorTool(value: string): DirectorToolDto {
  return DIRECTOR_TOOLS.find((tool) => tool.value === value)?.value ?? "lineart";
}

export function parseSafetyOverride(value: string): GallerySafetyOverrideDto | null {
  return value === "safe" || value === "sensitive" || value === "hidden" ? value : null;
}

export function clampDefry(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.min(5, Math.floor(value))) : 0;
}

function createId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}
