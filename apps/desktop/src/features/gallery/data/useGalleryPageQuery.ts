import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { runLoggedAction } from "@/app/logger";
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
      runLoggedAction("Apply gallery safety override", () => galleryApi.setSafetyOverride(request)),
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
    mutationFn: (request: DeleteGalleryItemsRequestDto) =>
      runLoggedAction("Delete gallery items", () => galleryApi.deleteItems(request)),
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
    mutationFn: (request: SaveResourceImageRequestDto) =>
      runLoggedAction("Save gallery image", () => desktopApi.saveResourceImage(request)),
  });
}

export function useGalleryImageReferenceMutation() {
  return useMutation({
    mutationFn: (request: GalleryImageReferenceRequestDto) =>
      runLoggedAction("Create gallery image reference", () => galleryApi.imageReference(request)),
  });
}
