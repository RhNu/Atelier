import { ChevronLeft, ChevronRight, Download, ImageIcon, RotateCcw, Trash2 } from "lucide-react";
import { useCallback, type ChangeEvent } from "react";

import { AppIconButton, AppPanel } from "../../../components/ui";
import type { GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto } from "../../../types";
import { GenerationResourceImage } from "./GenerationResourceImage";

const HISTORY_STATUS_OPTIONS: Array<{
  value: "all" | GenerationBatchHistoryStatusDto;
  label: string;
}> = [
  { value: "all", label: "All" },
  { value: "succeeded", label: "Succeeded" },
  { value: "partially_succeeded", label: "Partial" },
  { value: "running", label: "Running" },
  { value: "paused", label: "Paused" },
  { value: "failed", label: "Failed" },
  { value: "stopped", label: "Stopped" },
];

type HistoryStatusFilter = "all" | GenerationBatchHistoryStatusDto;

type GenerationHistoryRailProps = {
  batches: ReadonlyArray<GenerationHistoryBatchDto>;
  pending: boolean;
  error: string | null;
  selectedBatchId: string | null;
  statusFilter: HistoryStatusFilter;
  offset: number;
  limit: number;
  total: number;
  rerunPending: boolean;
  deletePending: boolean;
  exportPending: boolean;
  onSelect: (batchId: string) => void;
  onStatusFilterChange: (status: HistoryStatusFilter) => void;
  onPreviousPage: () => void;
  onNextPage: () => void;
  onRerunSelected: () => void;
  onDeleteSelected: () => void;
  onExportSelected: () => void;
};

export function GenerationHistoryRail({
  batches,
  pending,
  error,
  selectedBatchId,
  statusFilter,
  offset,
  limit,
  total,
  rerunPending,
  deletePending,
  exportPending,
  onSelect,
  onStatusFilterChange,
  onPreviousPage,
  onNextPage,
  onRerunSelected,
  onDeleteSelected,
  onExportSelected,
}: GenerationHistoryRailProps) {
  const selectedBatch = batches.find((batch) => batch.batch_id === selectedBatchId) ?? null;
  const canPrevious = offset > 0;
  const canNext = offset + limit < total;
  const handleStatusChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      if (isHistoryStatusFilter(event.target.value)) onStatusFilterChange(event.target.value);
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
          <h2 className="text-sm font-semibold text-white">History · batches</h2>
          <div className="flex items-center gap-1">
            <AppIconButton
              icon={RotateCcw}
              label="Rerun selected batch"
              disabled={!selectedBatch || rerunPending}
              onClick={onRerunSelected}
            />
            <AppIconButton
              icon={Download}
              label="Export selected batch as ZIP"
              disabled={!selectedBatch?.outputs.length || exportPending}
              onClick={onExportSelected}
            />
            <AppIconButton
              icon={Trash2}
              label="Delete selected batch history"
              variant="danger"
              disabled={!selectedBatch || deletePending}
              onClick={onDeleteSelected}
            />
          </div>
        </div>
        <label className="sr-only" htmlFor="generation-history-filter">
          Filter history batches
        </label>
        <select
          id="generation-history-filter"
          aria-label="Filter history batches"
          value={statusFilter}
          onChange={handleStatusChange}
          className="h-8 w-28 border border-app-border bg-app-surface px-2 text-sm text-app-text outline-none focus:border-brand-400"
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
        {!pending && !error && batches.length === 0 ? (
          <p className="p-4 text-center text-sm text-app-muted">No generation batches</p>
        ) : null}
        <div className="grid gap-1">
          {batches.map((batch) => (
            <GenerationHistoryBatch
              key={batch.batch_id}
              batch={batch}
              selected={selectedBatchId === batch.batch_id}
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

function GenerationHistoryBatch({
  batch,
  selected,
  onSelect,
}: {
  batch: GenerationHistoryBatchDto;
  selected: boolean;
  onSelect: (batchId: string) => void;
}) {
  const handleSelect = useCallback(() => onSelect(batch.batch_id), [batch.batch_id, onSelect]);
  return (
    <button
      type="button"
      onClick={handleSelect}
      className={[
        "grid grid-cols-[52px_minmax(0,1fr)] gap-2 border bg-app-surface/75 p-2 text-left transition-colors",
        selected ? "border-brand-400/70" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <span className="grid h-13 w-13 grid-cols-2 grid-rows-2 gap-px overflow-hidden border border-app-border bg-black/30">
        {batch.outputs.length ? (
          batch.outputs
            .slice(0, 4)
            .map((output, index) => (
              <GenerationResourceImage
                key={`${output.artifact_id}:${output.sample_index ?? index}`}
                resource={output.resource}
                alt={`Batch output ${index + 1}`}
                className="h-full min-h-0 w-full bg-app-panel text-[8px]"
                fallbackLabel=""
              />
            ))
        ) : (
          <span className="col-span-2 row-span-2 flex items-center justify-center text-app-muted">
            <ImageIcon aria-hidden="true" className="size-5" />
          </span>
        )}
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-semibold text-app-text">
          {batch.title ?? "Generation batch"}
        </span>
        <span className="mt-1 block text-xs text-app-muted">
          {batch.completed_request_count}/{batch.request_count} requests ·{" "}
          {batch.completed_sample_count}/{batch.expected_sample_count} samples
        </span>
        <span className={`mt-1 block text-[11px] ${statusTextClass(batch.status)}`}>
          {batch.status}
        </span>
        {batch.last_error ? (
          <span className="mt-1 block truncate text-[11px] text-rose-100">{batch.last_error}</span>
        ) : null}
      </span>
    </button>
  );
}

function isHistoryStatusFilter(value: string): value is HistoryStatusFilter {
  return HISTORY_STATUS_OPTIONS.some((option) => option.value === value);
}

function formatHistoryRange(offset: number, limit: number, total: number): string {
  if (total === 0) return "0 batches";
  return `${offset + 1}-${Math.min(offset + limit, total)} of ${total}`;
}

function statusTextClass(status: GenerationBatchHistoryStatusDto): string {
  if (status === "failed") return "text-rose-200";
  if (status === "succeeded") return "text-emerald-200";
  if (status === "partially_succeeded") return "text-amber-200";
  if (status === "running") return "text-brand-200";
  return "text-app-muted";
}
