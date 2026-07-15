import { EmptyState } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

import { formatError } from "../gallery-utils";
import { GalleryCard } from "./GalleryCard";

type GalleryGridProps = {
  isPending: boolean;
  isError: boolean;
  error: unknown;
  items: GalleryItemDto[];
  selectedItemId: string | null;
  blurSensitive: boolean;
  onSelect: (itemId: string) => void;
};

export function GalleryGrid({
  isPending,
  isError,
  error,
  items,
  selectedItemId,
  blurSensitive,
  onSelect,
}: GalleryGridProps) {
  if (isPending) {
    return <p className="text-sm text-app-muted">Loading gallery</p>;
  }

  if (isError) {
    return <EmptyState title="Gallery unavailable" description={formatError(error)} />;
  }

  if (items.length === 0) {
    return <EmptyState title="No matching images" />;
  }

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3">
      {items.map((item) => (
        <GalleryCard
          key={item.item_id}
          item={item}
          selected={item.item_id === selectedItemId}
          blurSensitive={blurSensitive}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}
