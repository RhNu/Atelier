import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  desktopApi,
  galleryApi,
  globalSettingsApi,
  queryKeys,
  resourceApi,
  resourceImageToDataUrl,
} from "@/platform/atelier";
import type {
  DeleteGalleryItemsRequestDto,
  GalleryQueryDto,
  GalleryImageReferenceRequestDto,
  ResourceRefDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "@/types";

export function useGalleryPageQuery(query: GalleryQueryDto) {
  return useQuery({
    queryKey: queryKeys.gallery.list(query),
    queryFn: () => galleryApi.list(query),
  });
}

export function useGallerySettingsQuery() {
  return useQuery({
    queryKey: queryKeys.app.globalSettings(),
    queryFn: () => globalSettingsApi.get(),
  });
}

export function useGalleryImageQuery(resource: ResourceRefDto | null) {
  return useQuery({
    queryKey: resource
      ? queryKeys.resource.image(resource)
      : ["workspace", "resource", "image", null],
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

export function useDeleteGalleryItemsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: DeleteGalleryItemsRequestDto) => galleryApi.deleteItems(request),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.history.root() }),
      ]);
    },
  });
}

export function useSaveGalleryImageMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImageRequestDto) => desktopApi.saveResourceImage(request),
  });
}

export function useGalleryImageReferenceMutation() {
  return useMutation({
    mutationFn: (request: GalleryImageReferenceRequestDto) => galleryApi.imageReference(request),
  });
}
