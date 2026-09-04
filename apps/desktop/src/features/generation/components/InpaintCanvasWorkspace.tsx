/* eslint-disable max-lines-per-function, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { Check, Eraser, Paintbrush, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppRangeField, AppSelect } from "@/components/ui";
import type { CommitGenerationCanvasResourcesResponseDto } from "@/types";

import { useResourceImageQuery } from "../data/useGenerationActions";
import type { GenerationI2iDraft, GenerationInpaintSessionDraft } from "../model/generation-draft";

type Tool = "brush" | "eraser";

export function InpaintCanvasWorkspace({
  i2i,
  size,
  commitPending,
  onCommit,
  onApply,
  onCancel,
}: {
  i2i: GenerationI2iDraft;
  size: { width: number; height: number };
  commitPending: boolean;
  onCommit: (request: {
    source_png_base64: string;
    region_to_replace_png_base64: string;
  }) => Promise<CommitGenerationCanvasResourcesResponseDto>;
  onApply: (
    image: CommitGenerationCanvasResourcesResponseDto["source"],
    inpaint: GenerationInpaintSessionDraft,
  ) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("generation");
  const sourceQuery = useResourceImageQuery(i2i.image);
  const maskQuery = useResourceImageQuery(i2i.inpaint?.regionToReplace ?? null);
  const sourceRef = useRef<HTMLCanvasElement>(null);
  const maskRef = useRef<HTMLCanvasElement>(null);
  const drawingRef = useRef(false);
  const lastPointRef = useRef<{ x: number; y: number } | null>(null);
  const [tool, setTool] = useState<Tool>("brush");
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [display, setDisplay] = useState(
    () =>
      i2i.inpaint?.display ?? {
        color: "#2563eb",
        opacity: 0.45,
        pattern: "solid" as const,
        showBorder: true,
        brushSize: 48,
      },
  );

  useEffect(() => {
    const source = sourceRef.current;
    const mask = maskRef.current;
    const sourceData = sourceQuery.data;
    if (!source || !mask || !sourceData) return;
    const image = new Image();
    image.onload = () => {
      source.width = size.width;
      source.height = size.height;
      mask.width = size.width;
      mask.height = size.height;
      const context = source.getContext("2d");
      if (context) {
        context.imageSmoothingEnabled = true;
        context.imageSmoothingQuality = "high";
        context.drawImage(image, 0, 0, size.width, size.height);
      }
      if (maskQuery.data) drawExistingMask(mask, maskQuery.data.image_base64);
    };
    image.src = `data:${sourceData.mime_type ?? "image/png"};base64,${sourceData.image_base64}`;
  }, [maskQuery.data, size.height, size.width, sourceQuery.data]);

  function point(event: PointerEvent<HTMLCanvasElement>) {
    const canvas = maskRef.current;
    if (!canvas) return null;
    const bounds = canvas.getBoundingClientRect();
    return {
      x: ((event.clientX - bounds.left) / bounds.width) * canvas.width,
      y: ((event.clientY - bounds.top) / bounds.height) * canvas.height,
    };
  }

  function draw(event: PointerEvent<HTMLCanvasElement>) {
    if (!drawingRef.current) return;
    const canvas = maskRef.current;
    const next = point(event);
    if (!canvas || !next) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const previous = lastPointRef.current ?? next;
    context.save();
    context.globalCompositeOperation = tool === "eraser" ? "destination-out" : "source-over";
    context.strokeStyle = display.color;
    context.lineWidth = display.brushSize;
    context.lineCap = "round";
    context.lineJoin = "round";
    context.beginPath();
    context.moveTo(previous.x, previous.y);
    context.lineTo(next.x, next.y);
    context.stroke();
    context.restore();
    lastPointRef.current = next;
    setDirty(true);
  }

  function startDrawing(event: PointerEvent<HTMLCanvasElement>) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drawingRef.current = true;
    lastPointRef.current = point(event);
    draw(event);
  }

  function stopDrawing() {
    drawingRef.current = false;
    lastPointRef.current = null;
  }

  function clearMask() {
    const canvas = maskRef.current;
    canvas?.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
    setDirty(true);
  }

  async function commit() {
    const source = sourceRef.current;
    const mask = maskRef.current;
    if (!source || !mask) return;
    setError(null);
    try {
      const committed = await onCommit({
        source_png_base64: dataUrlPayload(source.toDataURL("image/png")),
        region_to_replace_png_base64: binaryMaskBase64(mask),
      });
      onApply(committed.source, { regionToReplace: committed.region_to_replace, display });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  function cancel() {
    if (!dirty || window.confirm(t("discardInpaintChanges"))) onCancel();
  }

  return (
    <section className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)_auto] bg-app-panel">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-app-border px-3 py-2">
        <div>
          <h2 className="text-sm font-semibold text-app-text">{t("inpaintEditor")}</h2>
          <p className="text-[11px] text-app-muted">{t("inpaintSemanticHint")}</p>
        </div>
        <div className="flex items-center gap-1">
          <AppButton
            variant={tool === "brush" ? "primary" : "secondary"}
            className="h-8 px-2 text-xs"
            onClick={() => setTool("brush")}
          >
            <Paintbrush className="size-4" />
            {t("brush")}
          </AppButton>
          <AppButton
            variant={tool === "eraser" ? "primary" : "secondary"}
            className="h-8 px-2 text-xs"
            onClick={() => setTool("eraser")}
          >
            <Eraser className="size-4" />
            {t("eraser")}
          </AppButton>
          <AppButton variant="danger" className="h-8 px-2 text-xs" onClick={clearMask}>
            <Trash2 className="size-4" />
            {t("clearMask")}
          </AppButton>
        </div>
      </header>
      <div className="grid min-h-0 grid-cols-[minmax(0,1fr)_180px] overflow-hidden">
        <div className="grid min-h-0 place-items-center overflow-auto bg-black/30 p-4">
          <div
            className={
              display.showBorder
                ? "relative max-h-full max-w-full border border-blue-300"
                : "relative max-h-full max-w-full"
            }
          >
            <canvas ref={sourceRef} className="block max-h-[70vh] max-w-full" />
            <canvas
              ref={maskRef}
              aria-label={t("inpaintMaskCanvas")}
              className={
                display.pattern === "stripes"
                  ? "absolute inset-0 size-full cursor-crosshair bg-[repeating-linear-gradient(45deg,transparent,transparent_8px,rgba(37,99,235,.12)_8px,rgba(37,99,235,.12)_16px)]"
                  : "absolute inset-0 size-full cursor-crosshair"
              }
              style={{ opacity: display.opacity }}
              onPointerDown={startDrawing}
              onPointerMove={draw}
              onPointerUp={stopDrawing}
              onPointerCancel={stopDrawing}
            />
          </div>
        </div>
        <aside className="grid content-start gap-4 border-l border-app-border p-3">
          <label className="grid gap-1 text-xs text-app-muted">
            {t("maskColor")}
            <input
              type="color"
              value={display.color}
              onChange={(event) =>
                setDisplay((current) => ({ ...current, color: event.target.value }))
              }
            />
          </label>
          <AppRangeField
            label={t("maskOpacity")}
            value={display.opacity}
            valueText={`${Math.round(display.opacity * 100)}%`}
            min={0.1}
            max={1}
            step={0.05}
            onChange={(opacity) => setDisplay((current) => ({ ...current, opacity }))}
          />
          <AppRangeField
            label={t("brushSize")}
            value={display.brushSize}
            valueText={`${display.brushSize}px`}
            min={1}
            max={512}
            step={1}
            onChange={(brushSize) => setDisplay((current) => ({ ...current, brushSize }))}
          />
          <AppSelect
            aria-label={t("maskPattern")}
            value={display.pattern}
            options={[
              { value: "solid", label: t("solid") },
              { value: "stripes", label: t("stripes") },
            ]}
            onValueChange={(pattern) =>
              setDisplay((current) => ({
                ...current,
                pattern: pattern === "stripes" ? "stripes" : "solid",
              }))
            }
          />
          <label className="flex items-center justify-between gap-2 text-xs text-app-muted">
            {t("showMaskBorder")}
            <input
              type="checkbox"
              checked={display.showBorder}
              onChange={(event) =>
                setDisplay((current) => ({ ...current, showBorder: event.target.checked }))
              }
            />
          </label>
        </aside>
      </div>
      <footer className="flex items-center justify-between gap-2 border-t border-app-border px-3 py-2">
        <p className="text-xs text-rose-200">{error}</p>
        <div className="flex gap-2">
          <AppButton variant="secondary" onClick={cancel}>
            <X className="size-4" />
            {t("cancel")}
          </AppButton>
          <AppButton disabled={commitPending || !sourceQuery.data} onClick={() => void commit()}>
            <Check className="size-4" />
            {commitPending ? t("saving") : t("apply")}
          </AppButton>
        </div>
      </footer>
    </section>
  );
}

function drawExistingMask(canvas: HTMLCanvasElement, base64: string) {
  const image = new Image();
  image.onload = () => {
    const context = canvas.getContext("2d");
    if (!context) return;
    context.imageSmoothingEnabled = false;
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
  };
  image.src = `data:image/png;base64,${base64}`;
}
function binaryMaskBase64(mask: HTMLCanvasElement): string {
  const output = document.createElement("canvas");
  output.width = mask.width;
  output.height = mask.height;
  const context = output.getContext("2d");
  if (!context) return "";
  context.fillStyle = "black";
  context.fillRect(0, 0, output.width, output.height);
  const pixels = mask.getContext("2d")?.getImageData(0, 0, mask.width, mask.height);
  if (!pixels) return "";
  const binary = context.createImageData(mask.width, mask.height);
  for (let index = 0; index < pixels.data.length; index += 4) {
    const value = pixels.data[index + 3] ? 255 : 0;
    binary.data[index] = value;
    binary.data[index + 1] = value;
    binary.data[index + 2] = value;
    binary.data[index + 3] = 255;
  }
  context.putImageData(binary, 0, 0);
  return dataUrlPayload(output.toDataURL("image/png"));
}
function dataUrlPayload(value: string): string {
  return value.slice(value.indexOf(",") + 1);
}
