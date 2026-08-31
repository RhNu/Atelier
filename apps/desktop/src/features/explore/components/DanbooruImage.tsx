import type { DanbooruMediaVariantDto } from "@/types";

import { ExploreImage } from "./ExploreImage";

type Props = {
  postId: number;
  variant: DanbooruMediaVariantDto;
  alt: string;
  className: string;
  blurred: boolean;
  eager?: boolean;
};
export function DanbooruImage({ postId, variant, ...props }: Props) {
  return (
    <ExploreImage
      {...props}
      sourceId="danbooru_database"
      itemId={String(postId)}
      variant={variant === "preview" ? "thumbnail" : "preview"}
    />
  );
}
