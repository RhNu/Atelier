import type { ResourceRefDto } from "@/types";

export function isImportedImageResource(
  resource: ResourceRefDto | null,
): resource is ResourceRefDto {
  return resource?.variant_id === null && resource.id.startsWith("resource:import:");
}

export function uniqueImportedImageResources(
  resources: ReadonlyArray<ResourceRefDto | null>,
): ResourceRefDto[] {
  const unique = new Map<string, ResourceRefDto>();
  for (const resource of resources) {
    if (isImportedImageResource(resource)) {
      unique.set(resource.id, resource);
    }
  }
  return [...unique.values()];
}
