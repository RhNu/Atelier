import { useCallback } from "react";

import { SafetyBadge } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

import { effectiveSafetyLabel, preferredThumbnailResource } from "../gallery-utils";
import { GalleryItemImage } from "./GalleryItemImage";

type GalleryCardProps = {
  item: GalleryItemDto;
  selected: boolean;
  blurSensitive: boolean;
  onSelect: (itemId: string) => void;
};

export function GalleryCard({ item, selected, blurSensitive, onSelect }: GalleryCardProps) {
  const handleSelect = useCallback(() => onSelect(item.item_id), [item.item_id, onSelect]);

  return (
    <article
      className={[
        "border bg-app-surface transition-colors",
        selected ? "border-brand-400" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <button
        type="button"
        aria-label={`Select ${item.item_id}`}
        onClick={handleSelect}
        className="block w-full text-left"
      >
        <GalleryItemImage
          item={item}
          resource={preferredThumbnailResource(item)}
          alt={`${item.item_id} preview`}
          className="aspect-[4/5] w-full bg-app-bg"
          blurSensitive={blurSensitive}
        />
        <div className="grid gap-2 p-3">
          <div className="flex min-w-0 items-center justify-between gap-2">
            <p className="min-w-0 truncate text-sm font-semibold text-white">{item.item_id}</p>
            <SafetyBadge label={effectiveSafetyLabel(item)} />
          </div>
          <p className="truncate text-xs text-app-muted">{item.model_name ?? item.artifact_kind}</p>
        </div>
      </button>
    </article>
  );
}
