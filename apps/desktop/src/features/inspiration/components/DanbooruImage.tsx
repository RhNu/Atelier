import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ResourceImage } from "@/components/ui";
import type { DanbooruMediaVariantDto } from "@/types";

import { useDanbooruMediaQuery } from "../data/useDanbooruQueries";

type Props = {
  postId: number;
  variant: DanbooruMediaVariantDto;
  alt: string;
  className: string;
  blurred: boolean;
  eager?: boolean;
};

export function DanbooruImage({ postId, variant, alt, className, blurred, eager = false }: Props) {
  const { t } = useTranslation("inspiration");
  const root = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(eager);

  useEffect(() => {
    if (eager || visible || !root.current) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "240px" },
    );
    observer.observe(root.current);
    return () => observer.disconnect();
  }, [eager, visible]);

  const image = useDanbooruMediaQuery(postId, variant, eager || visible);
  return (
    <div ref={root} className={className}>
      <ResourceImage
        src={image.data ?? null}
        alt={alt}
        fallbackLabel={image.isError ? t("imageUnavailable") : t("loadingImage")}
        className={["size-full", blurred ? "blur-xl" : ""].join(" ")}
      />
    </div>
  );
}
