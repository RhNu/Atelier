import { EmptyState } from "@/components/ui";
import type { GalleryItemDto } from "@/types";

import { formatError } from "../gallery-utils";
import { GalleryCard } from "./GalleryCard";
import { GallerySelectionToolbar } from "./GallerySelectionToolbar";

type GalleryGridProps = {
  isPending: boolean;
  isError: boolean;
  error: unknown;
  items: GalleryItemDto[];
  selectedItemId: string | null;
  selectedItemIds: ReadonlySet<string>;
  blurSensitive: boolean;
  onSelect: (itemId: string) => void;
  onToggleSelection: (itemId: string, selected: boolean) => void;
  onToggleAll: (selected: boolean) => void;
  onDeleteSelected: () => void;
  deleting: boolean;
};

export function GalleryGrid({
  isPending,
  isError,
  error,
  items,
  selectedItemId,
  selectedItemIds,
  blurSensitive,
  onSelect,
  onToggleSelection,
  onToggleAll,
  onDeleteSelected,
  deleting,
}: GalleryGridProps) {
  const { t } = useTranslation("gallery");
  if (isPending) {
    return <p className="text-sm text-app-muted">{t("loading")}</p>;
  }

  if (isError) {
    return <EmptyState title={t("unavailable")} description={formatError(error)} />;
  }

  if (items.length === 0) {
    return <EmptyState title={t("noMatches")} />;
  }

  return (
    <div className="grid gap-3">
      <GallerySelectionToolbar
        itemCount={items.length}
        selectedCount={selectedItemIds.size}
        deleting={deleting}
        onToggleAll={onToggleAll}
        onDeleteSelected={onDeleteSelected}
      />
      <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3">
        {items.map((item) => (
          <GalleryCard
            key={item.item_id}
            item={item}
            selected={item.item_id === selectedItemId}
            batchSelected={selectedItemIds.has(item.item_id)}
            blurSensitive={blurSensitive}
            onSelect={onSelect}
            onToggleSelection={onToggleSelection}
          />
        ))}
      </div>
    </div>
  );
}
import { useTranslation } from "react-i18next";
