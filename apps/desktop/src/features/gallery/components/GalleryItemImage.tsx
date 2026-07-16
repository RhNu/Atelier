import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation("gallery");
  const imageQuery = useGalleryImageQuery(resource);
  const shouldBlur = blurSensitive && effectiveSafetyLabel(item) === "sensitive";

  return (
    <ResourceImage
      src={imageQuery.data ?? null}
      alt={alt}
      fallbackLabel={imageQuery.isError ? t("imageUnavailable") : t("loadingImage")}
      className={[className, shouldBlur ? "blur-md" : ""].join(" ")}
    />
  );
}
