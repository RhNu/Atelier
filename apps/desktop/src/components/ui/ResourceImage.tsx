import type { ImgHTMLAttributes } from "react";

type ResourceImageProps = Omit<ImgHTMLAttributes<HTMLImageElement>, "src"> & {
  src: string | null;
  fallbackLabel?: string;
};

export function ResourceImage({
  src,
  fallbackLabel = "No image",
  className = "",
  alt = "",
  ...props
}: ResourceImageProps) {
  if (!src) {
    return (
      <div
        className={[
          "flex items-center justify-center bg-app-surface text-xs text-app-muted",
          className,
        ].join(" ")}
      >
        {fallbackLabel}
      </div>
    );
  }

  return <img src={src} alt={alt} className={["object-contain", className].join(" ")} {...props} />;
}
