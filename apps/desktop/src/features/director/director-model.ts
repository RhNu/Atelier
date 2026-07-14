import type {
  DirectorToolDto,
  GallerySafetyOverrideDto,
  ImageInputDto,
  ResourceRefDto,
} from "../../types";

export type DirectorInput =
  | { kind: "resource"; resource: ResourceRefDto; label: string }
  | { kind: "inline"; imageBase64: string; src: string; label: string };

export const DIRECTOR_TOOLS: ReadonlyArray<{
  value: DirectorToolDto;
  label: string;
  description: string;
}> = [
  { value: "lineart", label: "Lineart", description: "Extract clean line art" },
  { value: "sketch", label: "Sketch", description: "Create a loose sketch pass" },
  { value: "bg_removal", label: "Background", description: "Remove the background" },
  { value: "declutter", label: "Declutter", description: "Clean visual noise" },
  { value: "colorize", label: "Colorize", description: "Add color with an optional prompt" },
  { value: "emotion", label: "Emotion", description: "Change expression from a prompt" },
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

export async function readClipboardImage(): Promise<{ imageBase64: string; src: string }> {
  const clipboard = globalThis.navigator?.clipboard;
  if (!clipboard || !("read" in clipboard)) {
    throw new Error("Clipboard images are unavailable in this environment");
  }
  const items = await clipboard.read();
  for (const item of items) {
    const mimeType = item.types.find((type) => type.startsWith("image/"));
    if (mimeType) {
      const blob = await item.getType(mimeType);
      const imageBase64 = await blobToBase64(blob);
      return { imageBase64, src: `data:${mimeType};base64,${imageBase64}` };
    }
  }
  throw new Error("Clipboard does not contain an image");
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      const result = typeof reader.result === "string" ? reader.result : "";
      resolve(result.split(",")[1] ?? "");
    });
    reader.addEventListener("error", () => reject(new Error("Unable to read pasted image")));
    reader.readAsDataURL(blob);
  });
}
