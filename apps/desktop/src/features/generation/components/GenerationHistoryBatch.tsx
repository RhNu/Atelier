import { ImageIcon } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import type { GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto } from "@/types";

import { translateGenerationStatus } from "../generation-status";
import { GenerationResourceImage } from "./GenerationResourceImage";

export function GenerationHistoryBatch({
  batch,
  selected,
  checked,
  onSelect,
  onToggleSelection,
}: {
  batch: GenerationHistoryBatchDto;
  selected: boolean;
  checked: boolean;
  onSelect: (batchId: string) => void;
  onToggleSelection: (batchId: string) => void;
}) {
  const { t } = useTranslation("generation");
  const handleSelect = useCallback(() => onSelect(batch.batch_id), [batch.batch_id, onSelect]);
  const handleToggleSelection = useCallback(
    () => onToggleSelection(batch.batch_id),
    [batch.batch_id, onToggleSelection],
  );
  const availableOutputs = batch.outputs.filter((output) => output.state === "available");
  return (
    <article
      className={[
        "relative aspect-square min-w-0 overflow-hidden border bg-app-surface/75 [content-visibility:auto]",
        selected ? "border-brand-300 ring-1 ring-brand-400/50" : "border-app-border",
        checked ? "bg-brand-500/10" : "",
      ].join(" ")}
    >
      <button
        type="button"
        onClick={handleSelect}
        aria-label={batch.title ?? t("generationBatch")}
        className="absolute inset-0 grid grid-cols-2 grid-rows-2 gap-px bg-black/30 text-left"
      >
        {availableOutputs.length ? (
          availableOutputs
            .slice(0, 4)
            .map((output, index) => (
              <GenerationResourceImage
                key={`${output.artifact_id}:${output.sample_index ?? index}`}
                resource={output.resource}
                alt={t("batchOutput", { index: index + 1 })}
                className={[
                  "h-full min-h-0 w-full bg-app-panel object-cover text-[8px]",
                  availableOutputs.length === 1 ? "col-span-2 row-span-2" : "",
                ].join(" ")}
                fallbackLabel=""
              />
            ))
        ) : (
          <span className="col-span-2 row-span-2 flex items-center justify-center text-app-muted">
            <ImageIcon aria-hidden="true" className="size-5" />
          </span>
        )}
        <span className="absolute inset-x-0 bottom-0 truncate bg-black/75 px-1.5 py-1 text-[11px] font-semibold text-white">
          {batch.title ?? t("generationBatch")}
        </span>
        <span
          className={`absolute top-1 right-1 bg-black/75 px-1 py-0.5 text-[9px] ${statusTextClass(batch.status)}`}
        >
          {translateGenerationStatus(t, batch.status)}
        </span>
        {batch.last_error ? (
          <span
            title={batch.last_error}
            className="absolute inset-x-1 bottom-7 truncate bg-rose-950/90 px-1 py-0.5 text-[9px] text-rose-100"
          >
            {batch.last_error}
          </span>
        ) : (
          <>
            <span className="absolute bottom-7 left-1 bg-black/75 px-1 py-0.5 text-[9px] text-white">
              {batch.available_sample_count}/{batch.expected_sample_count}
            </span>
            <span className="absolute right-1 bottom-7 bg-black/75 px-1 py-0.5 text-[9px] text-white">
              {batch.completed_request_count}/{batch.request_count}
            </span>
          </>
        )}
      </button>
      <label className="absolute top-1 left-1 grid size-5 place-items-center bg-black/75">
        <span className="sr-only">{t("selectBatchForCleanup")}</span>
        <input
          type="checkbox"
          aria-label={t("selectBatchForCleanup")}
          checked={checked}
          onChange={handleToggleSelection}
          className="size-3.5 accent-brand-400"
        />
      </label>
    </article>
  );
}

function statusTextClass(status: GenerationBatchHistoryStatusDto): string {
  if (status === "failed") return "text-rose-200";
  if (status === "succeeded") return "text-emerald-200";
  if (status === "partially_succeeded") return "text-amber-200";
  if (status === "running") return "text-brand-200";
  return "text-app-muted";
}
