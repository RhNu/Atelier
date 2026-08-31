import { useContext, useMemo, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ResourceImage } from "@/components/ui";
import type { ExploreSourceIdDto, ExploreMediaVariantDto } from "@/types";

import { useExploreMediaQuery } from "../data/useExploreQueries";
import { ExploreActiveContext } from "../state/explore-active";

type Props = {
  sourceId: ExploreSourceIdDto;
  itemId: string;
  variant: ExploreMediaVariantDto;
  alt: string;
  className: string;
  blurred: boolean;
  eager?: boolean;
};

export function ExploreImage({ eager = false, ...props }: Props) {
  const active = useContext(ExploreActiveContext);
  const root = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(eager);
  const { t } = useTranslation("explore");
  useEffect(() => {
    if (!active || eager || !root.current) return;
    const observer = new IntersectionObserver(
      (entries) => {
        setVisible(entries.some((entry) => entry.isIntersecting));
      },
      { rootMargin: "240px" },
    );
    observer.observe(root.current);
    return () => observer.disconnect();
  }, [active, eager]);
  return (
    <div ref={root} className={props.className}>
      {active && (eager || visible) ? (
        <LoadedImage {...props} />
      ) : (
        <ResourceImage
          src={null}
          alt={props.alt}
          fallbackLabel={t("loadingImage")}
          className="size-full"
        />
      )}
    </div>
  );
}

function LoadedImage({ sourceId, itemId, variant, alt, blurred }: Props) {
  const { t } = useTranslation("explore");
  const item = useMemo(() => ({ source_id: sourceId, item_id: itemId }), [sourceId, itemId]);
  const image = useExploreMediaQuery(item, variant);
  return (
    <ResourceImage
      src={image.data ?? null}
      alt={alt}
      fallbackLabel={image.isError ? t("imageUnavailable") : t("loadingImage")}
      className={["size-full", blurred ? "blur-xl" : ""].join(" ")}
    />
  );
}
