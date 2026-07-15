/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { ChevronLeft, ChevronRight, LoaderCircle } from "lucide-react";
import { useMemo, useState } from "react";

import { AppButton, AppModal } from "../../../components/ui";
import type { ImageModelDto, VibeDocumentEntryDto } from "../../../types";
import { useVibeDocumentsQuery } from "../data/useGenerationActions";
import { GenerationResourceThumbnail } from "./GenerationResourceThumbnail";
import { findVibeEncodingForModel } from "./vibe-guidance-model";

const PAGE_LIMIT = 32;

export function VibeLibraryDialog({
  open,
  model,
  onClose,
  onSelect,
}: {
  open: boolean;
  model: ImageModelDto;
  onClose: () => void;
  onSelect: (entry: VibeDocumentEntryDto) => void;
}) {
  const [offset, setOffset] = useState(0);
  const query = useVibeDocumentsQuery({ offset, limit: PAGE_LIMIT, include_hidden: false }, open);
  const entries = useMemo(
    () =>
      (query.data?.items ?? []).filter((entry) => findVibeEncodingForModel(entry, model) !== null),
    [model, query.data?.items],
  );
  const total = query.data?.total ?? 0;
  const canGoPrevious = offset > 0;
  const canGoNext = offset + PAGE_LIMIT < total;

  function close() {
    setOffset(0);
    onClose();
  }

  return (
    <AppModal open={open} title="Vibe library" onClose={close}>
      <div className="grid gap-3">
        {query.isPending ? (
          <div className="flex min-h-40 items-center justify-center gap-2 text-sm text-app-muted">
            <LoaderCircle aria-hidden="true" className="size-4 animate-spin" />
            Loading Vibes
          </div>
        ) : query.isError ? (
          <p className="border border-rose-500/40 bg-rose-950/30 p-3 text-sm text-rose-100">
            {formatError(query.error)}
          </p>
        ) : entries.length === 0 ? (
          <div className="grid min-h-40 place-items-center border border-dashed border-app-border text-sm text-app-muted">
            No compatible Vibes on this page.
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
            {entries.map((entry) => (
              <button
                key={entry.vibe_id}
                type="button"
                className="grid min-w-0 gap-2 border border-app-border bg-black/20 p-2 text-left hover:border-brand-400/60 hover:bg-app-surface"
                onClick={() => {
                  onSelect(entry);
                  close();
                }}
              >
                <GenerationResourceThumbnail
                  resource={entry.preview ?? entry.source_image}
                  alt={entry.display_name}
                  className="aspect-square w-full"
                />
                <span className="truncate text-xs font-semibold text-app-text">
                  {entry.display_name}
                </span>
              </button>
            ))}
          </div>
        )}

        <footer className="flex items-center justify-between border-t border-app-border pt-3 text-xs text-app-muted">
          <span>
            {total === 0
              ? "No Vibes"
              : `${offset + 1}–${Math.min(offset + PAGE_LIMIT, total)} of ${total}`}
          </span>
          <div className="flex items-center gap-2">
            <AppButton
              variant="ghost"
              className="h-8 px-2 text-xs"
              disabled={!canGoPrevious}
              onClick={() => setOffset((value) => Math.max(0, value - PAGE_LIMIT))}
            >
              <ChevronLeft aria-hidden="true" className="size-4" />
              Previous
            </AppButton>
            <AppButton
              variant="ghost"
              className="h-8 px-2 text-xs"
              disabled={!canGoNext}
              onClick={() => setOffset((value) => value + PAGE_LIMIT)}
            >
              Next
              <ChevronRight aria-hidden="true" className="size-4" />
            </AppButton>
          </div>
        </footer>
      </div>
    </AppModal>
  );
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Vibe library unavailable";
}
