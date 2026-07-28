import { Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";

export function GallerySelectionToolbar({
  itemCount,
  selectedCount,
  deleting,
  onToggleAll,
  onDeleteSelected,
}: {
  itemCount: number;
  selectedCount: number;
  deleting: boolean;
  onToggleAll: (selected: boolean) => void;
  onDeleteSelected: () => void;
}) {
  const { t } = useTranslation("gallery");
  const selectAllRef = useRef<HTMLInputElement>(null);
  const allSelected = itemCount > 0 && selectedCount === itemCount;

  useEffect(() => {
    if (selectAllRef.current) {
      selectAllRef.current.indeterminate = selectedCount > 0 && !allSelected;
    }
  }, [allSelected, selectedCount]);
  const handleToggleAll = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onToggleAll(event.target.checked),
    [onToggleAll],
  );

  return (
    <div className="flex min-h-9 items-center justify-between gap-3">
      <label className="flex items-center gap-2 text-sm text-app-text">
        <input
          ref={selectAllRef}
          type="checkbox"
          aria-label={t("selectAll")}
          checked={allSelected}
          disabled={itemCount === 0 || deleting}
          onChange={handleToggleAll}
        />
        {t("selectAll")}
      </label>
      <AppButton
        variant="danger"
        disabled={selectedCount === 0 || deleting}
        onClick={onDeleteSelected}
      >
        <Trash2 aria-hidden="true" className="size-4" />
        {t("deleteSelectedCount", { count: selectedCount })}
      </AppButton>
    </div>
  );
}
