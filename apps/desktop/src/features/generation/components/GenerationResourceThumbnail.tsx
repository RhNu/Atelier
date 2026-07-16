import { useTranslation } from "react-i18next";

import { resourceImageToDataUrl } from "@/platform/atelier";
import type { ResourceRefDto } from "@/types";

import { useResourceImageQuery } from "../data/useGenerationActions";

export function GenerationResourceThumbnail({
  resource,
  alt,
  className = "size-16",
}: {
  resource: ResourceRefDto | null;
  alt: string;
  className?: string;
}) {
  const { t } = useTranslation("generation");
  const query = useResourceImageQuery(resource);
  const src = query.data ? resourceImageToDataUrl(query.data) : null;
  return (
    <div
      className={[
        "grid shrink-0 place-items-center overflow-hidden border border-app-border bg-app-bg text-center text-[10px] text-app-muted",
        className,
      ].join(" ")}
    >
      {src ? (
        <img src={src} alt={alt} className="size-full object-cover" />
      ) : query.isError ? (
        t("unavailable")
      ) : query.isPending && resource ? (
        t("loading")
      ) : (
        t("noImage")
      )}
    </div>
  );
}
