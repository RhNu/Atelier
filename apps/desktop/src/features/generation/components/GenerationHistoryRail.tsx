import { RotateCcw } from "lucide-react";
import { useCallback } from "react";

import { AppIconButton, AppPanel, EmptyState } from "../../../components/ui";
import type { RunHistoryItemDto } from "../../../types";

type GenerationHistoryRailProps = {
  items: ReadonlyArray<RunHistoryItemDto>;
  pending: boolean;
  error: string | null;
  selectedItemId: string | null;
  onSelect: (itemId: string) => void;
};

export function GenerationHistoryRail({
  items,
  pending,
  error,
  selectedItemId,
  onSelect,
}: GenerationHistoryRailProps) {
  return (
    <AppPanel
      as="aside"
      aria-label="Generation history"
      className="flex min-h-0 flex-col overflow-hidden"
    >
      <header className="flex items-center justify-between border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">History</h2>
        <AppIconButton icon={RotateCcw} label="Rerun selected history item" disabled />
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-2">
        {pending ? <p className="p-3 text-sm text-app-muted">Loading history</p> : null}
        {error ? <p className="p-3 text-sm text-rose-100">{error}</p> : null}
        {!pending && !error && items.length === 0 ? <EmptyState title="No runs" /> : null}
        <div className="grid gap-2">
          {items.map((item) => (
            <GenerationHistoryItem
              key={item.run_id}
              item={item}
              selected={selectedItemId === item.run_id}
              onSelect={onSelect}
            />
          ))}
        </div>
      </div>
    </AppPanel>
  );
}

function GenerationHistoryItem({
  item,
  selected,
  onSelect,
}: {
  item: RunHistoryItemDto;
  selected: boolean;
  onSelect: (itemId: string) => void;
}) {
  const handleSelect = useCallback(() => {
    onSelect(item.run_id);
  }, [item.run_id, onSelect]);

  return (
    <button
      type="button"
      onClick={handleSelect}
      className={[
        "border bg-app-surface/75 p-3 text-left transition-colors",
        selected ? "border-brand-400/70" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <span className="block truncate text-sm font-semibold text-app-text">
        {item.title ?? item.run_id}
      </span>
      <span className="mt-1 block text-xs text-app-muted">{item.status}</span>
      {item.last_error ? (
        <span className="mt-2 block truncate text-xs text-rose-100">{item.last_error}</span>
      ) : null}
      <span className="mt-2 block text-xs text-app-muted">{item.outputs.length} outputs</span>
    </button>
  );
}
