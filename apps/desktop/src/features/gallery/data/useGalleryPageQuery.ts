import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  desktopApi,
  galleryApi,
  queryKeys,
  resourceApi,
  resourceImageToDataUrl,
  settingsApi,
} from "../../../platform/atelier";
import type {
  GalleryQueryDto,
  ResourceRefDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "../../../types";

export function useGalleryPageQuery(query: GalleryQueryDto) {
  return useQuery({
    queryKey: queryKeys.gallery.list(query),
    queryFn: () => galleryApi.list(query),
  });
}

export function useGallerySettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}

export function useGalleryImageQuery(resource: ResourceRefDto | null) {
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

export function useSetGallerySafetyOverrideMutation() {
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

export function useSaveGalleryImageMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImageRequestDto) => desktopApi.saveResourceImage(request),
  });
}
