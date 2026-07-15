import { ImageIcon, Save, Sparkles, Wand2 } from "lucide-react";
import { useCallback, type ReactNode } from "react";

import { AppButton, AppPanel, ResourceImage } from "../../../components/ui";
import type { GenerationStatusDto, ResourceImageDto } from "../../../types";
import type { GenerationPreview } from "../state/generation-event-store";

type GenerationPreviewStageProps = {
  preview: GenerationPreview | null;
  finalImage: ResourceImageDto | undefined;
  finalImagePending: boolean;
  finalImageError: string | null;
  status: GenerationStatusDto | undefined;
  statusError: string | null;
  lastError: string | null;
  filmstrip: ReadonlyArray<GenerationPreview>;
  savePending: boolean;
  handoffPending: boolean;
  compilePending: boolean;
  queueControls: ReactNode;
  onSelectPreview: (preview: GenerationPreview) => void;
  onSavePreview: () => void;
  onSendPreviewToDirector: () => void;
  onCompilePrompt: () => void;
};

export function GenerationPreviewStage({
  preview,
  finalImage,
  finalImagePending,
  finalImageError,
  status,
  statusError,
  lastError,
  filmstrip,
  savePending,
  handoffPending,
  compilePending,
  queueControls,
  onSelectPreview,
  onSavePreview,
  onSendPreviewToDirector,
  onCompilePrompt,
}: GenerationPreviewStageProps) {
  const src = previewSrc(preview, finalImage);
  const alt =
    preview?.kind === "resource" ? "Final generation preview" : "Active generation preview";
  const fallback = getPreviewFallback({
    finalImagePending,
    finalImageError,
    statusError,
  });
  const activityLabel = getActivityLabel(status);

  return (
    <AppPanel className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-3 py-2">
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Preview</p>
          <h2 className="text-base font-semibold text-white">
            {preview?.kind === "stream" ? "Streaming frame" : "Generation canvas"}
          </h2>
        </div>
        <div className="flex items-center gap-2">
          {lastError ? (
            <span className="max-w-48 truncate text-xs text-rose-100" title={lastError}>
              {lastError}
            </span>
          ) : null}
          {activityLabel ? (
            <span className="border border-brand-400/40 bg-brand-500/10 px-2 py-1 text-xs text-brand-100">
              {activityLabel}
            </span>
          ) : null}
          {filmstrip.length ? (
            <span className="text-xs text-app-muted">
              {filmstrip.length} {filmstrip.length === 1 ? "preview" : "previews"}
            </span>
          ) : null}
          <AppButton variant="ghost" onClick={onCompilePrompt} disabled={compilePending}>
            <Sparkles aria-hidden="true" className="size-4" />
            Compile
          </AppButton>
          {queueControls}
          <AppButton
            variant="secondary"
            onClick={onSavePreview}
            disabled={preview?.kind !== "resource" || savePending}
          >
            <Save aria-hidden="true" className="size-4" />
            Save
          </AppButton>
          <AppButton
            variant="ghost"
            onClick={onSendPreviewToDirector}
            disabled={preview?.kind !== "resource" || !preview.galleryItemId || handoffPending}
          >
            <Wand2 aria-hidden="true" className="size-4" />
            Director
          </AppButton>
        </div>
      </header>
      <div className="min-h-0 bg-black/30 p-2">
        {src ? (
          <ResourceImage src={src} alt={alt} className="h-full min-h-[360px] w-full" />
        ) : (
          <div className="flex h-full min-h-[360px] flex-col items-center justify-center bg-app-surface text-app-muted">
            <ImageIcon aria-hidden="true" className="mb-4 size-12 opacity-40" />
            <p className="text-sm">{fallback}</p>
          </div>
        )}
      </div>
      {filmstrip.length ? (
        <div className="flex gap-2 overflow-x-auto border-t border-app-border bg-app-panel/80 p-2">
          {filmstrip.map((item, index) => (
            <FilmstripButton
              key={filmstripKey(item)}
              item={item}
              index={index}
              selected={preview ? filmstripKey(preview) === filmstripKey(item) : false}
              onSelectPreview={onSelectPreview}
            />
          ))}
        </div>
      ) : null}
    </AppPanel>
  );
}

function FilmstripButton({
  item,
  index,
  selected,
  onSelectPreview,
}: {
  item: GenerationPreview;
  index: number;
  selected: boolean;
  onSelectPreview: (preview: GenerationPreview) => void;
}) {
  const handleClick = useCallback(() => {
    onSelectPreview(item);
  }, [item, onSelectPreview]);

  return (
    <button
      type="button"
      onClick={handleClick}
      className={[
        "h-16 w-20 shrink-0 border bg-black/30 text-xs text-app-muted",
        selected ? "border-brand-400" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      {item.kind === "stream" ? (
        <img
          src={item.src}
          alt={`Filmstrip frame ${index + 1}`}
          className="h-full w-full object-cover"
        />
      ) : (
        <span className="flex h-full w-full items-center justify-center px-1 text-center">
          Preview {index + 1}
        </span>
      )}
    </button>
  );
}

function filmstripKey(preview: GenerationPreview): string {
  if (preview.kind === "stream") {
    return [
      preview.kind,
      preview.batchId,
      preview.jobId,
      preview.sampleIndex,
      preview.stepIndex ?? "final",
      preview.generationId,
    ].join(":");
  }
  return [
    preview.kind,
    preview.batchId,
    preview.jobId,
    preview.sampleIndex,
    preview.artifactId,
  ].join(":");
}

function getPreviewFallback({
  finalImagePending,
  finalImageError,
  statusError,
}: {
  finalImagePending: boolean;
  finalImageError: string | null;
  statusError: string | null;
}): string {
  if (finalImageError) {
    return `Final image unavailable: ${finalImageError}`;
  }
  if (statusError) {
    return `Generation status unavailable: ${statusError}`;
  }
  if (finalImagePending) {
    return "Loading final image";
  }
  return "No active preview";
}

function getActivityLabel(status: GenerationStatusDto | undefined): string | null {
  const statuses = [status?.batch_status, status?.job_status].map((value) => value?.toLowerCase());
  if (statuses.includes("running")) {
    return "Generating";
  }
  if (statuses.includes("waiting")) {
    return "Queued";
  }
  if (statuses.includes("paused")) {
    return "Paused";
  }
  return null;
}

function previewSrc(
  preview: GenerationPreview | null,
  finalImage: ResourceImageDto | undefined,
): string | null {
  if (!preview) {
    return null;
  }
  if (preview.kind === "stream") {
    return preview.src;
  }
  if (!finalImage) {
    return null;
  }

  return `data:${finalImage.mime_type ?? "image/png"};base64,${finalImage.image_base64}`;
}
