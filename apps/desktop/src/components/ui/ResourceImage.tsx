import type { ImgHTMLAttributes } from "react";
import { useTranslation } from "react-i18next";

type ResourceImageProps = Omit<ImgHTMLAttributes<HTMLImageElement>, "src"> & {
  src: string | null;
  fallbackLabel?: string;
};

export function ResourceImage({
  src,
  fallbackLabel,
  className = "",
  alt = "",
  ...props
}: ResourceImageProps) {
  const { t } = useTranslation("common");
  if (!src) {
    return (
      <div
        className={[
          "flex items-center justify-center bg-app-surface text-xs text-app-muted",
          className,
        ].join(" ")}
      >
        {fallbackLabel ?? t("noImage")}
      </div>
    );
  }

  return <img src={src} alt={alt} className={["object-contain", className].join(" ")} {...props} />;
}
