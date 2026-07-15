import {
  ChevronLeft,
  ChevronRight,
  Download,
  ImageIcon,
  RotateCcw,
  Trash2,
  Wand2,
} from "lucide-react";
import type { ChangeEvent } from "react";
import { useCallback } from "react";

import { AppIconButton, AppPanel } from "../../../components/ui";
import type { RunHistoryItemDto, RunHistoryStatusDto } from "../../../types";

const HISTORY_STATUS_OPTIONS: Array<{ value: "all" | RunHistoryStatusDto; label: string }> = [
  { value: "all", label: "All" },
  { value: "succeeded", label: "Succeeded" },
  { value: "running", label: "Running" },
  { value: "failed", label: "Failed" },
  { value: "paused", label: "Paused" },
];

type GenerationHistoryRailProps = {
  items: ReadonlyArray<RunHistoryItemDto>;
  pending: boolean;
  error: string | null;
  selectedItemId: string | null;
  statusFilter: "all" | RunHistoryStatusDto;
  offset: number;
  limit: number;
  total: number;
  rerunPending: boolean;
  deletePending: boolean;
  exportPending: boolean;
  handoffPending: boolean;
  onSelect: (itemId: string) => void;
  onStatusFilterChange: (status: "all" | RunHistoryStatusDto) => void;
  onPreviousPage: () => void;
  onNextPage: () => void;
  onRerunSelected: () => void;
  onDeleteSelected: () => void;
  onExportSelected: () => void;
  onSendSelectedToDirector: () => void;
};

export function GenerationHistoryRail({
  items,
  pending,
  error,
  selectedItemId,
  statusFilter,
  offset,
  limit,
  total,
  rerunPending,
  deletePending,
  exportPending,
  handoffPending,
  onSelect,
  onStatusFilterChange,
  onPreviousPage,
  onNextPage,
  onRerunSelected,
  onDeleteSelected,
  onExportSelected,
  onSendSelectedToDirector,
}: GenerationHistoryRailProps) {
  const selectedItem = items.find((item) => item.run_id === selectedItemId) ?? null;
  const selectedHasOutput = Boolean(selectedItem?.outputs.length);
  const selectedHasGalleryItem = Boolean(selectedItem?.outputs.some((output) => output.item_id));
  const canPrevious = offset > 0;
  const canNext = offset + limit < total;
  const handleStatusChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const value = event.target.value;
      if (isHistoryStatusFilter(value)) {
        onStatusFilterChange(value);
      }
    },
    [onStatusFilterChange],
  );

  return (
    <AppPanel
      as="aside"
      aria-label="Generation history"
      className="flex h-full min-h-0 flex-col overflow-hidden"
    >
      <header className="grid gap-2 border-b border-app-border px-3 py-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-white">History</h2>
          <div className="flex items-center gap-1">
            <AppIconButton
              icon={RotateCcw}
              label="Rerun selected history item"
              disabled={!selectedItem || rerunPending}
              onClick={onRerunSelected}
            />
            <AppIconButton
              icon={Download}
              label="Export selected history output"
              disabled={!selectedHasOutput || exportPending}
              onClick={onExportSelected}
            />
            <AppIconButton
              icon={Wand2}
              label="Send selected history output to Director"
              disabled={!selectedHasGalleryItem || handoffPending}
              onClick={onSendSelectedToDirector}
            />
            <AppIconButton
              icon={Trash2}
              label="Delete selected history item"
              disabled={!selectedItem || deletePending}
              onClick={onDeleteSelected}
            />
          </div>
        </div>
        <label className="sr-only" htmlFor="generation-history-filter">
          Filter history
        </label>
        <select
          id="generation-history-filter"
          aria-label="Filter history"
          value={statusFilter}
          onChange={handleStatusChange}
          className="h-8 w-24 border border-app-border bg-app-surface px-2 text-sm text-app-text outline-none focus:border-brand-400"
        >
          {HISTORY_STATUS_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-1">
        {pending ? <p className="p-2 text-sm text-app-muted">Loading history</p> : null}
        {error ? <p className="p-2 text-sm text-rose-100">{error}</p> : null}
        {!pending && !error && items.length === 0 ? (
          <p className="p-4 text-center text-sm text-app-muted">No runs</p>
        ) : null}
        <div className="grid gap-1">
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
      <footer className="flex items-center justify-between border-t border-app-border p-1 text-xs text-app-muted">
        <span>{formatHistoryRange(offset, limit, total)}</span>
        <div className="flex items-center gap-1">
          <AppIconButton
            icon={ChevronLeft}
            label="Previous history page"
            disabled={!canPrevious || pending}
            onClick={onPreviousPage}
          />
          <AppIconButton
            icon={ChevronRight}
            label="Next history page"
            disabled={!canNext || pending}
            onClick={onNextPage}
          />
        </div>
      </footer>
    </AppPanel>
  );
}

function isHistoryStatusFilter(value: string): value is "all" | RunHistoryStatusDto {
  return HISTORY_STATUS_OPTIONS.some((option) => option.value === value);
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
        "border bg-app-surface/75 p-2 text-left transition-colors",
        selected ? "border-brand-400/70" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <span className="grid grid-cols-[44px_minmax(0,1fr)] gap-2">
        <span className="flex h-11 w-11 items-center justify-center border border-app-border bg-black/30 text-app-muted">
          <ImageIcon aria-hidden="true" className="size-5" />
        </span>
        <span className="min-w-0">
          <span className="block truncate text-sm font-semibold text-app-text">
            {item.title ?? (item.kind === "director" ? "Director result" : "Generation run")}
          </span>
          <span className="mt-1 block text-xs text-app-muted">
            {item.status} · {item.outputs.length} {item.outputs.length === 1 ? "output" : "outputs"}
          </span>
        </span>
      </span>
      {item.last_error ? (
        <span className="mt-2 block truncate text-xs text-rose-100">{item.last_error}</span>
      ) : null}
    </button>
  );
}

function formatHistoryRange(offset: number, limit: number, total: number): string {
  if (total === 0) {
    return "0 runs";
  }
  return `${offset + 1}-${Math.min(offset + limit, total)} of ${total}`;
}
