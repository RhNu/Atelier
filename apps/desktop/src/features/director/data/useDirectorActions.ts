import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  desktopApi,
  directorApi,
  galleryApi,
  queryKeys,
  resourceApi,
  resourceImageToDataUrl,
} from "../../../platform/atelier";
import type {
  ImageResourceKindDto,
  ResourceRefDto,
  RunDirectorToolRequestDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "../../../types";

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
    mutationFn: async () => {
      const [imported] = await desktopApi.pickAndImportImageResources(kind, { extensions: [] });
      return imported?.resource ?? null;
    },
  });
}

export function useRunDirectorToolMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: RunDirectorToolRequestDto) => directorApi.runTool(request),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.history.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
      ]);
    },
  });
}

export function useSaveDirectorImageMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImageRequestDto) => desktopApi.saveResourceImage(request),
  });
}

export function useSetDirectorSafetyOverrideMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: SetGallerySafetyOverrideRequestDto) =>
      galleryApi.setSafetyOverride(request),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
      ]);
    },
  });
}
