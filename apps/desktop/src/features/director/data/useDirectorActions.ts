import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { runLoggedAction } from "@/app/logger";
import {
  desktopApi,
  directorApi,
  galleryApi,
  queryKeys,
  resourceApi,
  uniqueImportedImageResources,
  resourceImageToDataUrl,
} from "@/platform/atelier";
import type {
  ImageResourceKindDto,
  ResourceRefDto,
  RunDirectorToolRequestDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "@/types";

export function useDirectorImageQuery(resource: ResourceRefDto | null) {
  return useQuery({
    queryKey: resource ? queryKeys.resource.image(resource) : ["resource", "image", null],
    queryFn: async () => {
      if (!resource) {
        throw new Error("resource is required");
      }
      return resourceImageToDataUrl(await resourceApi.image({ resource }));
    },
    enabled: Boolean(resource),
  });
}

export function usePickDirectorImageMutation(kind: ImageResourceKindDto = "source_image") {
  return useMutation({
    mutationFn: () =>
      runLoggedAction("Pick Director input image", async () => {
        const [imported, ...unused] = await desktopApi.pickAndImportImageResources(kind, {
          extensions: [],
        });
        const unusedResources = unused.map((item) => item.resource);
        if (unusedResources.length > 0) {
          await resourceApi.releaseImportedImages({ resources: unusedResources });
        }
        return imported?.resource ?? null;
      }),
  });
}

export function useReleaseDirectorImagesMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (resources: ReadonlyArray<ResourceRefDto | null>) =>
      runLoggedAction("Release Director image resources", async () => {
        const imported = uniqueImportedImageResources(resources);
        if (imported.length === 0) return null;
        return resourceApi.releaseImportedImages({ resources: imported });
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() });
    },
  });
}

export function useRunDirectorToolMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: RunDirectorToolRequestDto) =>
      runLoggedAction("Run Director tool", () => directorApi.runTool(request)),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.history.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.account.activeSummary() }),
      ]);
    },
  });
}

export function useSaveDirectorImageMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImageRequestDto) =>
      runLoggedAction("Save Director image", () => desktopApi.saveResourceImage(request)),
  });
}

export function useSetDirectorSafetyOverrideMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: SetGallerySafetyOverrideRequestDto) =>
      runLoggedAction("Apply Director safety override", () =>
        galleryApi.setSafetyOverride(request),
      ),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
      ]);
    },
  });
}
