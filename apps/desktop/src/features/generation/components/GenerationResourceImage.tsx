import type { ImgHTMLAttributes } from "react";

import { ResourceImage } from "../../../components/ui";
import type { ResourceRefDto } from "../../../types";
import { useResourceImageQuery } from "../data/useGenerationActions";
import { formatGenerationError } from "../generation-page-utils";

type GenerationResourceImageProps = Omit<
  ImgHTMLAttributes<HTMLImageElement>,
  "src" | "resource"
> & {
  resource: ResourceRefDto | null;
  fallbackLabel?: string;
};

export function GenerationResourceImage({
  resource,
  fallbackLabel = "No final image",
  ...props
}: GenerationResourceImageProps) {
  const imageQuery = useResourceImageQuery(resource);
  const label = imageQuery.isError
    ? `Image unavailable: ${formatGenerationError(imageQuery.error)}`
    : imageQuery.isPending && resource
      ? "Loading image"
      : fallbackLabel;
  const src = imageQuery.data
    ? `data:${imageQuery.data.mime_type ?? "image/png"};base64,${imageQuery.data.image_base64}`
    : null;

  return <ResourceImage {...props} src={src} fallbackLabel={label} />;
}
