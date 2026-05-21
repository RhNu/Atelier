import { useQuery } from "@tanstack/react-query";

import { galleryApi, queryKeys } from "../../../platform/atelier";
import type { GalleryQueryDto } from "../../../types";

const galleryQuery: GalleryQueryDto = {
  offset: 0,
  limit: 30,
  artifact_kind: null,
  source_kind: null,
  manual_safety_override: null,
};

export function useGalleryPageQuery() {
  return useQuery({
    queryKey: queryKeys.gallery.list(galleryQuery),
    queryFn: () => galleryApi.list(galleryQuery),
  });
}
