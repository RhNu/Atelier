/* eslint-disable max-lines */
import {
  Activity,
  Download,
  Grid2X2,
  History,
  ImageIcon,
  ListOrdered,
  RadioTower,
  RotateCcw,
  Save,
  Sparkles,
  Trash2,
  Wand2,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";

import { AppButton, AppIconButton, AppModal, AppPanel, ResourceImage } from "@/components/ui";

import type {
  GenerationBatchView,
  GenerationRequestUnit,
  GenerationSampleSlot,
} from "../model/generation-preview-model";
import type { GenerationFocusMode } from "../state/generation-event-store";
import { GenerationResourceImage } from "./GenerationResourceImage";

type GenerationPreviewStageProps = {
  batch: GenerationBatchView | null;
  selectedRequest: GenerationRequestUnit | null;
  focusedSampleIndex: number | null;
  focusMode: GenerationFocusMode;
  isViewingLive: boolean;
  liveBatchAvailable: boolean;
  statusError: string | null;
  lastError: string | null;
  savePending: boolean;
  zipPending: boolean;
  handoffPending: boolean;
  rerunPending: boolean;
  deletePending: boolean;
  compilePending: boolean;
  queueControls: ReactNode;
  onSelectRequest: (jobId: string) => void;
  onFocusSample: (jobId: string, sampleIndex: number) => void;
  onShowRequestGrid: () => void;
  onResumeLive: () => void;
  onSaveSample: () => void;
  onSendSampleToDirector: () => void;
  onExportRequest: () => void;
  onRerunRequest: () => void;
  onDeleteRequest: () => void;
  onCompilePrompt: () => void;
};

const CROPPED_PREVIEW_STYLE: CSSProperties = { objectFit: "cover" };

export function GenerationPreviewStage({
  batch,
  selectedRequest,
  focusedSampleIndex,
  focusMode,
  isViewingLive,
  liveBatchAvailable,
  statusError,
  lastError,
  savePending,
  zipPending,
  handoffPending,
  rerunPending,
  deletePending,
  compilePending,
  queueControls,
  onSelectRequest,
  onFocusSample,
  onShowRequestGrid,
  onResumeLive,
  onSaveSample,
  onSendSampleToDirector,
  onExportRequest,
  onRerunRequest,
  onDeleteRequest,
  onCompilePrompt,
}: GenerationPreviewStageProps) {
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const focusedSample =
    focusedSampleIndex === null
      ? null
      : (selectedRequest?.samples.find((sample) => sample.sampleIndex === focusedSampleIndex) ??
        null);
  useEffect(() => setLightboxOpen(false), [focusedSampleIndex, selectedRequest?.jobId]);
  const closeLightbox = useCallback(() => setLightboxOpen(false), []);
  const openLightbox = useCallback(() => {
    if (focusedSample?.resource || focusedSample?.streamSrc) setLightboxOpen(true);
  }, [focusedSample]);
  const showResume = liveBatchAvailable && (!isViewingLive || focusMode === "pin");

  return (
    <AppPanel className="grid h-full min-h-0 grid-rows-[auto_auto_minmax(0,1fr)] overflow-hidden">
      <PreviewHeader
        batch={batch}
        isViewingLive={isViewingLive}
        lastError={lastError}
        showResume={showResume}
        compilePending={compilePending}
        queueControls={queueControls}
        onResumeLive={onResumeLive}
        onCompilePrompt={onCompilePrompt}
      />
      <RequestCursorStrip
        batch={batch}
        selectedRequest={selectedRequest}
        onSelectRequest={onSelectRequest}
      />
      <RequestPreviewSurface
        selectedRequest={selectedRequest}
        focusedSample={focusedSample}
        statusError={statusError}
        savePending={savePending}
        zipPending={zipPending}
        handoffPending={handoffPending}
        rerunPending={rerunPending}
        deletePending={deletePending}
        onFocusSample={onFocusSample}
        onShowRequestGrid={onShowRequestGrid}
        onOpenLightbox={openLightbox}
        onSaveSample={onSaveSample}
        onSendSampleToDirector={onSendSampleToDirector}
        onExportRequest={onExportRequest}
        onRerunRequest={onRerunRequest}
        onDeleteRequest={onDeleteRequest}
      />
      <SampleLightbox
        open={lightboxOpen}
        request={selectedRequest}
        sample={focusedSample}
        onClose={closeLightbox}
      />
    </AppPanel>
  );
}

function PreviewHeader({
  batch,
  isViewingLive,
  lastError,
  showResume,
  compilePending,
  queueControls,
  onResumeLive,
  onCompilePrompt,
}: Pick<
  GenerationPreviewStageProps,
  | "batch"
  | "isViewingLive"
  | "lastError"
  | "compilePending"
  | "queueControls"
  | "onResumeLive"
  | "onCompilePrompt"
> & { showResume: boolean }) {
  const PreviewModeIcon = isViewingLive ? Activity : History;
  const previewModeLabel = isViewingLive ? "Live preview" : "History preview";
  const requestCount = batch?.requests.length ?? 0;
  const requestCountLabel = `${requestCount} request${requestCount === 1 ? "" : "s"}`;
  return (
    <header className="flex min-h-10 items-center justify-between gap-2 border-b border-app-border px-2 py-1">
      <div className="flex min-w-0 items-center gap-1.5">
        <span
          title={previewModeLabel}
          className="flex size-7 shrink-0 items-center justify-center text-brand-200"
        >
          <PreviewModeIcon aria-label={previewModeLabel} className="size-4" />
        </span>
        <h2 className="truncate text-sm font-semibold text-white">
          {batch ? "Batch" : "Generation canvas"}
        </h2>
        {batch ? (
          <span
            aria-label={requestCountLabel}
            title={requestCountLabel}
            className="inline-flex shrink-0 items-center gap-1 text-xs text-app-muted"
          >
            <ListOrdered aria-hidden="true" className="size-3.5" />
            <span>{requestCount}</span>
          </span>
        ) : null}
      </div>
      <div className="flex items-center gap-1">
        {lastError ? (
          <span className="max-w-40 truncate text-xs text-rose-100" title={lastError}>
            {lastError}
          </span>
        ) : null}
        {showResume ? (
          <AppIconButton
            icon={RadioTower}
            label={isViewingLive ? "Follow latest request" : "Return to live batch"}
            size="sm"
            onClick={onResumeLive}
          />
        ) : null}
        <AppButton
          variant="ghost"
          className="h-8 gap-1 px-2 text-xs"
          onClick={onCompilePrompt}
          disabled={compilePending}
        >
          <Sparkles aria-hidden="true" className="size-4" />
          Compile
        </AppButton>
        {queueControls}
      </div>
    </header>
  );
}

function RequestCursorStrip({
  batch,
  selectedRequest,
  onSelectRequest,
}: Pick<GenerationPreviewStageProps, "batch" | "selectedRequest" | "onSelectRequest">) {
  const cursorRef = useRef<HTMLDivElement>(null);
  const selectedRequestIndex = batch?.requests.findIndex(
    (request) => request.jobId === selectedRequest?.jobId,
  );
  useEffect(() => {
    if (selectedRequestIndex === undefined || selectedRequestIndex < 0) return;
    const selectedElement = cursorRef.current?.children.item(selectedRequestIndex);
    if (!(selectedElement instanceof HTMLElement) || !selectedElement.scrollIntoView) return;
    selectedElement.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "nearest" });
  }, [selectedRequestIndex]);
  return (
    <div
      ref={cursorRef}
      aria-label="Generation requests"
      className="flex min-h-20 gap-1.5 overflow-x-auto border-b border-app-border bg-app-panel/80 p-1.5"
    >
      {batch?.requests.map((request) => (
        <RequestCursorUnit
          key={request.jobId}
          request={request}
          selected={request.jobId === selectedRequest?.jobId}
          onSelect={onSelectRequest}
        />
      )) ?? <p className="p-2 text-sm text-app-muted">No request units</p>}
    </div>
  );
}

function RequestPreviewSurface({
  selectedRequest,
  focusedSample,
  statusError,
  savePending,
  zipPending,
  handoffPending,
  rerunPending,
  deletePending,
  onFocusSample,
  onShowRequestGrid,
  onOpenLightbox,
  onSaveSample,
  onSendSampleToDirector,
  onExportRequest,
  onRerunRequest,
  onDeleteRequest,
}: Pick<
  GenerationPreviewStageProps,
  | "selectedRequest"
  | "statusError"
  | "savePending"
  | "zipPending"
  | "handoffPending"
  | "rerunPending"
  | "deletePending"
  | "onFocusSample"
  | "onShowRequestGrid"
  | "onSaveSample"
  | "onSendSampleToDirector"
  | "onExportRequest"
  | "onRerunRequest"
  | "onDeleteRequest"
> & { focusedSample: GenerationSampleSlot | null; onOpenLightbox: () => void }) {
  const requestHasOutput = selectedRequest?.samples.some((sample) => sample.resource) ?? false;
  const canMutateRequest = Boolean(selectedRequest?.runId);
  return (
    <div className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] bg-black/30">
      <div className="flex min-h-11 items-center justify-between gap-2 border-b border-app-border px-3 py-1">
        <div className="min-w-0">
          <span className="text-sm font-semibold text-app-text">
            {selectedRequest ? `Request ${selectedRequest.requestIndex + 1}` : "No request"}
          </span>
          {selectedRequest ? (
            <span className="ml-2 text-xs text-app-muted">
              {selectedRequest.status} · {selectedRequest.expectedSamples} sample
              {selectedRequest.expectedSamples === 1 ? "" : "s"}
            </span>
          ) : null}
        </div>
        <div className="flex items-center gap-1">
          {focusedSample ? (
            <AppIconButton icon={Grid2X2} label="Back to sample grid" onClick={onShowRequestGrid} />
          ) : null}
          <AppIconButton
            icon={Download}
            label="Export request as ZIP"
            disabled={!requestHasOutput || zipPending}
            onClick={onExportRequest}
          />
          <AppIconButton
            icon={RotateCcw}
            label="Rerun request"
            disabled={!canMutateRequest || rerunPending}
            onClick={onRerunRequest}
          />
          <AppIconButton
            icon={Trash2}
            label="Delete request history"
            variant="danger"
            disabled={!canMutateRequest || deletePending}
            onClick={onDeleteRequest}
          />
          <AppIconButton
            icon={Save}
            label="Save selected sample"
            disabled={!focusedSample?.resource || savePending}
            onClick={onSaveSample}
          />
          <AppIconButton
            icon={Wand2}
            label="Send selected sample to Director"
            disabled={!focusedSample?.resource || !focusedSample.galleryItemId || handoffPending}
            onClick={onSendSampleToDirector}
          />
        </div>
      </div>
      <div className="min-h-0 p-2">
        {focusedSample && selectedRequest ? (
          <FocusedSample
            request={selectedRequest}
            sample={focusedSample}
            onOpenLightbox={onOpenLightbox}
          />
        ) : selectedRequest ? (
          <SampleGrid request={selectedRequest} onFocusSample={onFocusSample} />
        ) : (
          <PreviewEmptyState message={statusError ?? "No active preview"} />
        )}
      </div>
    </div>
  );
}

function SampleLightbox({
  open,
  request,
  sample,
  onClose,
}: {
  open: boolean;
  request: GenerationRequestUnit | null;
  sample: GenerationSampleSlot | null;
  onClose: () => void;
}) {
  return (
    <AppModal
      open={open}
      title={
        request && sample
          ? `Request ${request.requestIndex + 1} · Sample ${sample.sampleIndex + 1}`
          : "Generation sample"
      }
      onClose={onClose}
    >
      {sample ? (
        <SampleVisual
          sample={sample}
          alt={`Generation sample ${sample.sampleIndex + 1}`}
          className="max-h-[72svh] w-full bg-black/40"
        />
      ) : null}
    </AppModal>
  );
}

function RequestCursorUnit({
  request,
  selected,
  onSelect,
}: {
  request: GenerationRequestUnit;
  selected: boolean;
  onSelect: (jobId: string) => void;
}) {
  const handleSelect = useCallback(() => onSelect(request.jobId), [onSelect, request.jobId]);
  return (
    <button
      type="button"
      onClick={handleSelect}
      className={[
        "grid w-24 shrink-0 grid-rows-[minmax(0,1fr)_auto] gap-0.5 border p-1 text-left",
        selected
          ? "border-brand-400 bg-brand-500/10"
          : "border-app-border bg-app-surface/70 hover:border-brand-400/60",
      ].join(" ")}
    >
      <span
        className={`grid h-12 min-h-0 gap-px overflow-hidden bg-black/40 ${requestCursorGridClass(request.samples.length)}`}
      >
        {request.samples.slice(0, 4).map((sample, index) => (
          <SampleVisual
            key={sample.sampleIndex}
            sample={sample}
            alt={`Request ${request.requestIndex + 1} sample ${sample.sampleIndex + 1}`}
            className={requestCursorSampleClass(request.samples.length, index)}
            crop
          />
        ))}
      </span>
      <span className="flex items-center justify-between gap-1 text-[10px] text-app-muted">
        <span>R{request.requestIndex + 1}</span>
        <span className={statusTextClass(request.status)}>{shortStatus(request.status)}</span>
      </span>
    </button>
  );
}

function SampleGrid({
  request,
  onFocusSample,
}: {
  request: GenerationRequestUnit;
  onFocusSample: (jobId: string, sampleIndex: number) => void;
}) {
  return (
    <div className={sampleGridClass(request.samples.length)}>
      {request.samples.map((sample) => (
        <SampleGridButton
          key={sample.sampleIndex}
          request={request}
          sample={sample}
          onFocusSample={onFocusSample}
        />
      ))}
    </div>
  );
}

function SampleGridButton({
  request,
  sample,
  onFocusSample,
}: {
  request: GenerationRequestUnit;
  sample: GenerationSampleSlot;
  onFocusSample: (jobId: string, sampleIndex: number) => void;
}) {
  const handleFocus = useCallback(
    () => onFocusSample(request.jobId, sample.sampleIndex),
    [onFocusSample, request.jobId, sample.sampleIndex],
  );
  return (
    <button
      type="button"
      aria-label={`Focus sample ${sample.sampleIndex + 1}`}
      onClick={handleFocus}
      className="group relative min-h-0 overflow-hidden border border-app-border bg-black/35 hover:border-brand-400/70"
    >
      <SampleVisual
        sample={sample}
        alt={`Generation sample ${sample.sampleIndex + 1}`}
        className="h-full min-h-[220px] w-full"
      />
      <span className="absolute right-1 bottom-1 border border-app-border bg-black/75 px-1.5 py-0.5 text-[10px] text-app-text">
        S{sample.sampleIndex + 1} · {sample.state}
      </span>
    </button>
  );
}

function FocusedSample({
  request,
  sample,
  onOpenLightbox,
}: {
  request: GenerationRequestUnit;
  sample: GenerationSampleSlot;
  onOpenLightbox: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={`Open sample ${sample.sampleIndex + 1} lightbox`}
      onClick={onOpenLightbox}
      disabled={!sample.resource && !sample.streamSrc}
      className="relative h-full w-full overflow-hidden border border-app-border bg-black/35 disabled:cursor-default"
    >
      <SampleVisual
        sample={sample}
        alt={`Request ${request.requestIndex + 1} sample ${sample.sampleIndex + 1}`}
        className="h-full min-h-[360px] w-full"
      />
      <span className="absolute right-2 bottom-2 border border-app-border bg-black/75 px-2 py-1 text-xs text-app-text">
        Sample {sample.sampleIndex + 1} · {sample.state}
      </span>
    </button>
  );
}

function SampleVisual({
  sample,
  alt,
  className,
  crop = false,
}: {
  sample: GenerationSampleSlot;
  alt: string;
  className: string;
  crop?: boolean;
}) {
  const style = crop ? CROPPED_PREVIEW_STYLE : undefined;
  if (sample.resource) {
    return (
      <GenerationResourceImage
        resource={sample.resource}
        alt={alt}
        className={className}
        style={style}
        fallbackLabel="Final image unavailable"
      />
    );
  }
  if (sample.streamSrc) {
    return <ResourceImage src={sample.streamSrc} alt={alt} className={className} style={style} />;
  }
  return <ResourceImage src={null} alt="" className={className} fallbackLabel={sample.state} />;
}

function PreviewEmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full min-h-[360px] flex-col items-center justify-center bg-app-surface text-app-muted">
      <ImageIcon aria-hidden="true" className="mb-4 size-12 opacity-40" />
      <p className="text-sm">{message}</p>
    </div>
  );
}

function sampleGridClass(count: number): string {
  const columns = count === 1 ? "grid-cols-1" : "grid-cols-2";
  const rows = count <= 2 ? "grid-rows-1" : "grid-rows-2";
  return `grid h-full min-h-0 gap-2 ${columns} ${rows}`;
}

function requestCursorGridClass(count: number): string {
  if (count <= 1) return "grid-cols-1 grid-rows-1";
  if (count === 2) return "grid-cols-2 grid-rows-1";
  return "grid-cols-2 grid-rows-2";
}

function requestCursorSampleClass(count: number, index: number): string {
  const spanClass = count === 3 && index === 0 ? "row-span-2" : "";
  return `h-full min-h-0 w-full bg-app-panel text-[9px] ${spanClass}`;
}

function shortStatus(status: string): string {
  return status === "partially_succeeded" ? "partial" : status;
}

function statusTextClass(status: string): string {
  if (status === "failed") return "text-rose-200";
  if (status === "succeeded" || status === "complete") return "text-emerald-200";
  if (status === "partially_succeeded" || status === "partial") return "text-amber-200";
  if (status === "running") return "text-brand-200";
  return "text-app-muted";
}
