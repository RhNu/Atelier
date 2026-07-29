import {
  CheckCheck,
  ChevronLeft,
  ChevronRight,
  Download,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton, AppPanel, AppSelect } from "@/components/ui";
import type { GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto } from "@/types";

import { GenerationHistoryBatch } from "./GenerationHistoryBatch";

type HistoryStatusFilter = "all" | GenerationBatchHistoryStatusDto;

const HISTORY_STATUS_OPTIONS = [
  { value: "all", labelKey: "historyStatus.all" },
  { value: "succeeded", labelKey: "historyStatus.succeeded" },
  { value: "partially_succeeded", labelKey: "historyStatus.partially_succeeded" },
  { value: "running", labelKey: "historyStatus.running" },
  { value: "paused", labelKey: "historyStatus.paused" },
  { value: "failed", labelKey: "historyStatus.failed" },
  { value: "stopped", labelKey: "historyStatus.stopped" },
] as const satisfies ReadonlyArray<{
  value: HistoryStatusFilter;
  labelKey: `historyStatus.${string}`;
}>;

type GenerationHistoryRailProps = {
  batches: ReadonlyArray<GenerationHistoryBatchDto>;
  pending: boolean;
  error: string | null;
  selectedBatchId: string | null;
  selectedBatchIds: ReadonlySet<string>;
  statusFilter: HistoryStatusFilter;
  offset: number;
  limit: number;
  total: number;
  rerunPending: boolean;
  deletePending: boolean;
  exportPending: boolean;
  onSelect: (batchId: string) => void;
  onToggleSelection: (batchId: string) => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
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
  selectedBatchIds,
  statusFilter,
  offset,
  limit,
  total,
  rerunPending,
  deletePending,
  exportPending,
  onSelect,
  onToggleSelection,
  onSelectAll,
  onClearSelection,
  onStatusFilterChange,
  onPreviousPage,
  onNextPage,
  onRerunSelected,
  onDeleteSelected,
  onExportSelected,
}: GenerationHistoryRailProps) {
  const { t } = useTranslation("generation");
  const statusOptions = useMemo(
    () =>
      HISTORY_STATUS_OPTIONS.map((option) => ({
        value: option.value,
        label: t(option.labelKey),
      })),
    [t],
  );
  const selectedBatch = batches.find((batch) => batch.batch_id === selectedBatchId) ?? null;
  const canPrevious = offset > 0;
  const canNext = offset + limit < total;
  const handleStatusChange = useCallback(
    (value: string) => {
      if (isHistoryStatusFilter(value)) onStatusFilterChange(value);
    },
    [onStatusFilterChange],
  );

  return (
    <AppPanel
      as="aside"
      aria-label={t("history")}
      className="flex h-full min-h-0 flex-col overflow-hidden"
    >
      <header className="flex min-h-10 items-center justify-between gap-2 border-b border-app-border px-2 py-1">
        <label className="sr-only" htmlFor="generation-history-filter">
          {t("filterHistory")}
        </label>
        <AppSelect
          id="generation-history-filter"
          aria-label={t("filterHistory")}
          value={statusFilter}
          onValueChange={handleStatusChange}
          className="h-7 w-28 px-2 pr-6 text-xs"
          options={statusOptions}
        />
        <div className="flex items-center gap-1">
          <AppIconButton
            icon={CheckCheck}
            label={t("selectAllBatches")}
            size="sm"
            disabled={batches.length === 0}
            onClick={onSelectAll}
          />
          <AppIconButton
            icon={X}
            label={t("clearBatchSelection")}
            size="sm"
            disabled={selectedBatchIds.size === 0}
            onClick={onClearSelection}
          />
          <AppIconButton
            icon={RotateCcw}
            label={t("rerunSelectedBatch")}
            size="sm"
            disabled={!selectedBatch || rerunPending}
            onClick={onRerunSelected}
          />
          <AppIconButton
            icon={Download}
            label={t("exportSelectedBatch")}
            size="sm"
            disabled={!selectedBatch?.outputs.length || exportPending}
            onClick={onExportSelected}
          />
          <AppIconButton
            icon={Trash2}
            label={t("deleteSelectedBatches")}
            size="sm"
            variant="danger"
            disabled={selectedBatchIds.size === 0 || deletePending}
            onClick={onDeleteSelected}
          />
        </div>
      </header>
      {selectedBatchIds.size > 0 ? (
        <div className="border-b border-app-border bg-brand-500/10 px-2 py-1 text-[11px] text-brand-100">
          {t("selectedBatchCount", { count: selectedBatchIds.size })}
        </div>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto p-1">
        {pending ? <p className="p-2 text-sm text-app-muted">{t("loadingHistory")}</p> : null}
        {error ? <p className="p-2 text-sm text-rose-100">{error}</p> : null}
        {!pending && !error && batches.length === 0 ? (
          <p className="p-4 text-center text-sm text-app-muted">{t("noBatches")}</p>
        ) : null}
        <div className="grid grid-cols-2 gap-1">
          {batches.map((batch) => (
            <GenerationHistoryBatch
              key={batch.batch_id}
              batch={batch}
              selected={selectedBatchId === batch.batch_id}
              checked={selectedBatchIds.has(batch.batch_id)}
              onSelect={onSelect}
              onToggleSelection={onToggleSelection}
            />
          ))}
        </div>
      </div>
      <footer className="flex items-center justify-between border-t border-app-border p-1 text-xs text-app-muted">
        <span>{formatHistoryRange(offset, limit, total)}</span>
        <div className="flex items-center gap-1">
          <AppIconButton
            icon={ChevronLeft}
            label={t("previousHistoryPage")}
            size="sm"
            disabled={!canPrevious || pending}
            onClick={onPreviousPage}
          />
          <AppIconButton
            icon={ChevronRight}
            label={t("nextHistoryPage")}
            size="sm"
            disabled={!canNext || pending}
            onClick={onNextPage}
          />
        </div>
      </footer>
    </AppPanel>
  );
}

function isHistoryStatusFilter(value: string): value is HistoryStatusFilter {
  return HISTORY_STATUS_OPTIONS.some((option) => option.value === value);
}

function formatHistoryRange(offset: number, limit: number, total: number): string {
  if (total === 0) return "0 batches";
  return `${offset + 1}-${Math.min(offset + limit, total)} of ${total}`;
}
