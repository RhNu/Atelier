import { useCallback, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import { SafetyBadge } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

import {
  displayGalleryArtifactKind,
  displayGalleryModelName,
  effectiveSafetyLabel,
  preferredThumbnailResource,
} from "../gallery-utils";
import { GalleryItemImage } from "./GalleryItemImage";

type GalleryCardProps = {
  item: GalleryItemDto;
  selected: boolean;
  batchSelected: boolean;
  blurSensitive: boolean;
  onSelect: (itemId: string) => void;
  onToggleSelection: (itemId: string, selected: boolean) => void;
};

export function GalleryCard({
  item,
  selected,
  batchSelected,
  blurSensitive,
  onSelect,
  onToggleSelection,
}: GalleryCardProps) {
  const { t } = useTranslation("gallery");
  const handleSelect = useCallback(() => onSelect(item.item_id), [item.item_id, onSelect]);
  const handleToggleSelection = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onToggleSelection(item.item_id, event.target.checked),
    [item.item_id, onToggleSelection],
  );
  const safetyLabel = effectiveSafetyLabel(item);
  const sourceLabel =
    displayGalleryModelName(item.model_name) ?? displayGalleryArtifactKind(item.artifact_kind, t);

  return (
    <article
      className={[
        "relative border bg-app-surface transition-colors",
        selected ? "border-brand-400" : "border-app-border hover:border-brand-400/60",
        batchSelected ? "ring-1 ring-brand-300/80" : "",
      ].join(" ")}
    >
      <label className="absolute top-2 left-2 z-10 grid size-7 place-items-center border border-app-border bg-black/70">
        <input
          type="checkbox"
          aria-label={t("selectForBatch", { id: item.item_id })}
          checked={batchSelected}
          onChange={handleToggleSelection}
        />
      </label>
      <button
        type="button"
        aria-label={t("selectImage", { id: item.item_id })}
        onClick={handleSelect}
        className="block w-full text-left"
      >
        <GalleryItemImage
          item={item}
          resource={preferredThumbnailResource(item)}
          alt={t("galleryImageAlt")}
          className="aspect-[4/5] w-full bg-app-bg"
          blurSensitive={blurSensitive}
        />
        <div className="grid gap-2 p-3">
          <div className="flex min-w-0 items-center justify-between gap-2">
            <p className="min-w-0 truncate text-xs text-app-muted">{sourceLabel}</p>
            <SafetyBadge
              label={safetyLabel}
              displayLabel={safetyLabel === "unknown" ? t("unknown") : t(safetyLabel)}
            />
          </div>
        </div>
      </button>
    </article>
  );
}
