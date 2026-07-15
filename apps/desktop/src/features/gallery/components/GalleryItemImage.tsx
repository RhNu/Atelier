import { ResourceImage } from "@/components/ui";
import type { GalleryItemDto, ResourceRefDto } from "@/types";

import { useGalleryImageQuery } from "../data/useGalleryPageQuery";
import { effectiveSafetyLabel } from "../gallery-utils";

type GalleryItemImageProps = {
  item: GalleryItemDto;
  resource: ResourceRefDto;
  alt: string;
  className: string;
  blurSensitive: boolean;
};

export function GalleryItemImage({
  item,
  resource,
  alt,
  className,
  blurSensitive,
}: GalleryItemImageProps) {
  const imageQuery = useGalleryImageQuery(resource);
  const shouldBlur = blurSensitive && effectiveSafetyLabel(item) === "sensitive";

  return (
    <ResourceImage
      src={imageQuery.data ?? null}
      alt={alt}
      fallbackLabel={imageQuery.isError ? "Image unavailable" : "Loading image"}
      className={[className, shouldBlur ? "blur-md" : ""].join(" ")}
    />
  );
}
