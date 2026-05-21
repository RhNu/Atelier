import type { ResourceImageDto } from "../../types";

const objectUrls = new Set<string>();

export function resourceImageToDataUrl(image: ResourceImageDto): string {
  const mimeType = image.mime_type ?? "image/png";
  return `data:${mimeType};base64,${image.image_base64}`;
}

export function resourceImageToObjectUrl(image: ResourceImageDto): string {
  const mimeType = image.mime_type ?? "image/png";
  const binary = atob(image.image_base64);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  const url = URL.createObjectURL(new Blob([bytes], { type: mimeType }));
  objectUrls.add(url);
  return url;
}

export function revokeResourceImageObjectUrl(url: string): void {
  if (!objectUrls.delete(url)) {
    return;
  }
  URL.revokeObjectURL(url);
}
