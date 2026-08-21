import { useQuery } from "@tanstack/react-query";

import { generationApi } from "@/platform/atelier";
import type { ImageModelDescriptorDto, ImageModelDto } from "@/types";

export const imageModelCatalogQueryKey = ["generation", "image-model-catalog"] as const;

export function useImageModelCatalog() {
  return useQuery({
    queryKey: imageModelCatalogQueryKey,
    queryFn: generationApi.listModels,
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
  });
}

export function findModelDescriptor(
  catalog: readonly ImageModelDescriptorDto[] | undefined,
  model: ImageModelDto,
) {
  return catalog?.find((descriptor) => descriptor.model === model);
}
