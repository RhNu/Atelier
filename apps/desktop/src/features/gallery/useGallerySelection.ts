import { useCallback, useEffect, useMemo, useState } from "react";

import type { GalleryItemDto } from "@/types";

export function useGallerySelection(items: GalleryItemDto[]) {
  const [selectedItemIds, setSelectedItemIds] = useState<Set<string>>(() => new Set());
  const visibleItemIds = useMemo(() => items.map((item) => item.item_id), [items]);
  const visibleItemIdSet = useMemo(() => new Set(visibleItemIds), [visibleItemIds]);
  const visibleSelectedItemIds = useMemo(
    () => new Set([...selectedItemIds].filter((itemId) => visibleItemIdSet.has(itemId))),
    [selectedItemIds, visibleItemIdSet],
  );
  const selectedIds = useMemo(
    () => visibleItemIds.filter((itemId) => visibleSelectedItemIds.has(itemId)),
    [visibleItemIds, visibleSelectedItemIds],
  );

  useEffect(() => {
    setSelectedItemIds((current) => {
      const next = new Set([...current].filter((itemId) => visibleItemIdSet.has(itemId)));
      return next.size === current.size ? current : next;
    });
  }, [visibleItemIdSet]);

  const toggleItem = useCallback((itemId: string, selected: boolean) => {
    setSelectedItemIds((current) => {
      const next = new Set(current);
      if (selected) {
        next.add(itemId);
      } else {
        next.delete(itemId);
      }
      return next;
    });
  }, []);
  const toggleAll = useCallback(
    (selected: boolean) => {
      setSelectedItemIds(selected ? new Set(visibleItemIds) : new Set());
    },
    [visibleItemIds],
  );
  const clear = useCallback(() => setSelectedItemIds(new Set()), []);

  return {
    clear,
    selectedIds,
    selectedItemIds: visibleSelectedItemIds,
    toggleAll,
    toggleItem,
  };
}
