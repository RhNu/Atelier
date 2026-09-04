/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { Check, Eraser, ImagePlus, Paintbrush, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppRangeField, AppSelect } from "@/components/ui";
import type { CommitGenerationCanvasResourcesResponseDto } from "@/types";

import { loadGenerationResourceImage, useResourceImageQuery } from "../data/useGenerationActions";
import type {
  GenerationFocusRegionDraft,
  GenerationI2iDraft,
  GenerationInpaintSessionDraft,
  GenerationReferenceInsetDraft,
} from "../model/generation-draft";

type Tool = "brush" | "eraser";

export function InpaintCanvasWorkspace({
  i2i,
  size,
  commitPending,
  onCommit,
  onPickImageResources,
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
  onPickImageResources: () => Promise<CommitGenerationCanvasResourcesResponseDto["source"][]>;
  onApply: (
    image: CommitGenerationCanvasResourcesResponseDto["source"],
    inpaint: GenerationInpaintSessionDraft,
    size: { width: number; height: number },
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
  const [canvasSize, setCanvasSize] = useState(size);
  const [resizeSize, setResizeSize] = useState(size);
  const [anchor, setAnchor] = useState("center");
  const [focus, setFocus] = useState<GenerationFocusRegionDraft | null>(i2i.inpaint?.focus ?? null);
  const [referenceInsets, setReferenceInsets] = useState(() => i2i.inpaint?.referenceInsets ?? []);
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
      source.width = canvasSize.width;
      source.height = canvasSize.height;
      mask.width = canvasSize.width;
      mask.height = canvasSize.height;
      const context = source.getContext("2d");
      if (context) {
        context.imageSmoothingEnabled = true;
        context.imageSmoothingQuality = "high";
        context.drawImage(image, 0, 0, canvasSize.width, canvasSize.height);
      }
      if (maskQuery.data) drawExistingMask(mask, maskQuery.data.image_base64);
    };
    image.src = `data:${sourceData.mime_type ?? "image/png"};base64,${sourceData.image_base64}`;
  }, [canvasSize.height, canvasSize.width, maskQuery.data, sourceQuery.data]);

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

  function resizeCanvas() {
    const source = sourceRef.current;
    const mask = maskRef.current;
    if (!source || !mask) return;
    const width = legalDimension(resizeSize.width);
    const height = legalDimension(resizeSize.height);
    const [anchorX, anchorY] = anchorFactors(anchor);
    const offsetX = Math.round((width - source.width) * anchorX);
    const offsetY = Math.round((height - source.height) * anchorY);
    const oldSource = copyCanvas(source);
    const oldMask = copyCanvas(mask);
    source.width = width;
    source.height = height;
    const sourceContext = source.getContext("2d");
    if (!sourceContext) return;
    sourceContext.fillStyle = "white";
    sourceContext.fillRect(0, 0, width, height);
    sourceContext.drawImage(oldSource, offsetX, offsetY);
    mask.width = width;
    mask.height = height;
    const maskContext = mask.getContext("2d");
    if (!maskContext) return;
    if (width > oldMask.width || height > oldMask.height) {
      maskContext.fillStyle = display.color;
      maskContext.fillRect(0, 0, width, height);
      maskContext.clearRect(offsetX, offsetY, oldMask.width, oldMask.height);
    }
    maskContext.drawImage(oldMask, offsetX, offsetY);
    setCanvasSize({ width, height });
    setResizeSize({ width, height });
    setFocus(null);
    setDirty(true);
  }

  async function commit() {
    const source = sourceRef.current;
    const mask = maskRef.current;
    if (!source || !mask) return;
    setError(null);
    try {
      if (referenceInsets.length) {
        await flattenReferenceInsets(source, mask, referenceInsets, display.color);
      }
      if (focus && !hasPaintedPixels(mask)) {
        const context = mask.getContext("2d");
        if (context) {
          context.fillStyle = display.color;
          context.fillRect(
            focus.x * mask.width,
            focus.y * mask.height,
            focus.width * mask.width,
            focus.height * mask.height,
          );
        }
      }
      const committed = await onCommit({
        source_png_base64: dataUrlPayload(source.toDataURL("image/png")),
        region_to_replace_png_base64: binaryMaskBase64(mask),
      });
      onApply(
        committed.source,
        {
          regionToReplace: committed.region_to_replace,
          display,
          focus,
          referenceInsets,
        },
        canvasSize,
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function addReferenceInsets() {
    const resources = await onPickImageResources();
    setReferenceInsets((current) => [
      ...current,
      ...resources.map((image, index) => ({
        id: `reference-inset-${Date.now()}-${index}`,
        image,
        x: 0.15 + ((current.length + index) % 4) * 0.08,
        y: 0.15 + ((current.length + index) % 4) * 0.08,
        width: 0.35,
        height: 0.35,
        borderEnabled: true,
        borderWidth: 4,
      })),
    ]);
    if (resources.length) setDirty(true);
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
            {focus ? (
              <div
                className="pointer-events-none absolute border-2 border-amber-300 bg-amber-300/10"
                style={{
                  left: `${focus.x * 100}%`,
                  top: `${focus.y * 100}%`,
                  width: `${focus.width * 100}%`,
                  height: `${focus.height * 100}%`,
                }}
              />
            ) : null}
            {referenceInsets.map((inset) => (
              <ReferenceInsetLayer key={inset.id} inset={inset} />
            ))}
          </div>
        </div>
        <aside className="grid content-start gap-4 overflow-auto border-l border-app-border p-3">
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
          <label className="flex items-center justify-between gap-2 text-xs text-app-muted">
            {t("focusedInpaint")}
            <input
              type="checkbox"
              checked={Boolean(focus)}
              onChange={(event) =>
                setFocus(
                  event.target.checked
                    ? { x: 0.25, y: 0.25, width: 0.5, height: 0.5, minimumContextArea: 0.25 }
                    : null,
                )
              }
            />
          </label>
          {focus ? (
            <>
              <AppRangeField
                label="X"
                value={focus.x}
                valueText={focus.x.toFixed(2)}
                min={0}
                max={1 - focus.width}
                step={0.01}
                onChange={(x) => setFocus({ ...focus, x })}
              />
              <AppRangeField
                label="Y"
                value={focus.y}
                valueText={focus.y.toFixed(2)}
                min={0}
                max={1 - focus.height}
                step={0.01}
                onChange={(y) => setFocus({ ...focus, y })}
              />
              <AppRangeField
                label={t("focusWidth")}
                value={focus.width}
                valueText={focus.width.toFixed(2)}
                min={0.05}
                max={1 - focus.x}
                step={0.01}
                onChange={(width) => setFocus({ ...focus, width })}
              />
              <AppRangeField
                label={t("focusHeight")}
                value={focus.height}
                valueText={focus.height.toFixed(2)}
                min={0.05}
                max={1 - focus.y}
                step={0.01}
                onChange={(height) => setFocus({ ...focus, height })}
              />
              <AppRangeField
                label={t("minimumContextArea")}
                value={focus.minimumContextArea}
                valueText={`${Math.round(focus.minimumContextArea * 100)}%`}
                min={0}
                max={1}
                step={0.05}
                onChange={(minimumContextArea) => setFocus({ ...focus, minimumContextArea })}
              />
            </>
          ) : null}
          <div className="grid gap-2 border-t border-app-border pt-3">
            <span className="text-xs font-medium text-app-text">{t("resizeCanvas")}</span>
            <input
              aria-label={t("canvasWidth")}
              className="rounded border border-app-border bg-app-bg px-2 py-1 text-xs"
              type="number"
              min={64}
              max={1600}
              step={64}
              value={resizeSize.width}
              onChange={(event) =>
                setResizeSize((current) => ({ ...current, width: Number(event.target.value) }))
              }
            />
            <input
              aria-label={t("canvasHeight")}
              className="rounded border border-app-border bg-app-bg px-2 py-1 text-xs"
              type="number"
              min={64}
              max={1600}
              step={64}
              value={resizeSize.height}
              onChange={(event) =>
                setResizeSize((current) => ({ ...current, height: Number(event.target.value) }))
              }
            />
            <AppSelect
              aria-label={t("canvasAnchor")}
              value={anchor}
              options={[
                { value: "top-left", label: t("anchor.top-left") },
                { value: "top-center", label: t("anchor.top-center") },
                { value: "top-right", label: t("anchor.top-right") },
                { value: "center-left", label: t("anchor.center-left") },
                { value: "center-center", label: t("anchor.center-center") },
                { value: "center-right", label: t("anchor.center-right") },
                { value: "bottom-left", label: t("anchor.bottom-left") },
                { value: "bottom-center", label: t("anchor.bottom-center") },
                { value: "bottom-right", label: t("anchor.bottom-right") },
              ]}
              onValueChange={setAnchor}
            />
            <AppButton variant="secondary" className="h-8 text-xs" onClick={resizeCanvas}>
              {t("applyResize")}
            </AppButton>
          </div>
          <div className="grid gap-2 border-t border-app-border pt-3">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium text-app-text">{t("referenceInpainting")}</span>
              <AppButton
                variant="secondary"
                className="h-8 px-2 text-xs"
                onClick={() => void addReferenceInsets()}
              >
                <ImagePlus className="size-4" />
                {t("addReferenceInset")}
              </AppButton>
            </div>
            {referenceInsets.map((inset, index) => (
              <div key={inset.id} className="grid gap-2 border border-app-border p-2">
                <span className="text-xs text-app-text">
                  {t("referenceInset", { index: index + 1 })}
                </span>
                <AppRangeField
                  label="X"
                  value={inset.x}
                  valueText={inset.x.toFixed(2)}
                  min={0}
                  max={1 - inset.width}
                  step={0.01}
                  onChange={(x) =>
                    setReferenceInsets((items) => patchInset(items, inset.id, { x }))
                  }
                />
                <AppRangeField
                  label="Y"
                  value={inset.y}
                  valueText={inset.y.toFixed(2)}
                  min={0}
                  max={1 - inset.height}
                  step={0.01}
                  onChange={(y) =>
                    setReferenceInsets((items) => patchInset(items, inset.id, { y }))
                  }
                />
                <AppRangeField
                  label={t("insetSize")}
                  value={inset.width}
                  valueText={`${Math.round(inset.width * 100)}%`}
                  min={0.05}
                  max={Math.min(1 - inset.x, 1 - inset.y)}
                  step={0.01}
                  onChange={(width) =>
                    setReferenceInsets((items) =>
                      patchInset(items, inset.id, { width, height: width }),
                    )
                  }
                />
                <label className="flex justify-between text-xs text-app-muted">
                  {t("insetBorder")}
                  <input
                    type="checkbox"
                    checked={inset.borderEnabled}
                    onChange={(event) =>
                      setReferenceInsets((items) =>
                        patchInset(items, inset.id, { borderEnabled: event.target.checked }),
                      )
                    }
                  />
                </label>
                <AppRangeField
                  label={t("borderWidth")}
                  value={inset.borderWidth}
                  valueText={`${inset.borderWidth}px`}
                  min={0}
                  max={32}
                  step={1}
                  onChange={(borderWidth) =>
                    setReferenceInsets((items) => patchInset(items, inset.id, { borderWidth }))
                  }
                />
                <AppButton
                  variant="danger"
                  className="h-8 text-xs"
                  onClick={() =>
                    setReferenceInsets((items) => items.filter((item) => item.id !== inset.id))
                  }
                >
                  <Trash2 className="size-4" />
                  {t("remove")}
                </AppButton>
              </div>
            ))}
          </div>
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

function ReferenceInsetLayer({ inset }: { inset: GenerationReferenceInsetDraft }) {
  const query = useResourceImageQuery(inset.image);
  return query.data ? (
    <img
      src={`data:${query.data.mime_type ?? "image/png"};base64,${query.data.image_base64}`}
      alt=""
      className="pointer-events-none absolute object-fill"
      style={{
        left: `${inset.x * 100}%`,
        top: `${inset.y * 100}%`,
        width: `${inset.width * 100}%`,
        height: `${inset.height * 100}%`,
        border: inset.borderEnabled ? `${inset.borderWidth}px solid black` : undefined,
      }}
    />
  ) : null;
}

function patchInset(
  insets: GenerationReferenceInsetDraft[],
  id: string,
  patch: Partial<GenerationReferenceInsetDraft>,
) {
  return insets.map((inset) => (inset.id === id ? { ...inset, ...patch } : inset));
}

async function flattenReferenceInsets(
  source: HTMLCanvasElement,
  mask: HTMLCanvasElement,
  insets: GenerationReferenceInsetDraft[],
  maskColor: string,
) {
  const sourceContext = source.getContext("2d");
  const maskContext = mask.getContext("2d");
  if (!sourceContext || !maskContext) return;
  sourceContext.fillStyle = "white";
  sourceContext.fillRect(0, 0, source.width, source.height);
  maskContext.fillStyle = maskColor;
  maskContext.fillRect(0, 0, mask.width, mask.height);
  for (const inset of insets) {
    const resource = await loadGenerationResourceImage(inset.image);
    const image = await loadHtmlImage(resource.image_base64, resource.mime_type ?? "image/png");
    const x = inset.x * source.width;
    const y = inset.y * source.height;
    const width = inset.width * source.width;
    const height = inset.height * source.height;
    sourceContext.drawImage(image, x, y, width, height);
    if (inset.borderEnabled && inset.borderWidth > 0) {
      sourceContext.strokeStyle = "black";
      sourceContext.lineWidth = inset.borderWidth;
      sourceContext.strokeRect(x, y, width, height);
    }
    maskContext.clearRect(x, y, width, height);
  }
}

function loadHtmlImage(base64: string, mimeType: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Unable to decode reference inset"));
    image.src = `data:${mimeType};base64,${base64}`;
  });
}

function copyCanvas(source: HTMLCanvasElement): HTMLCanvasElement {
  const copy = document.createElement("canvas");
  copy.width = source.width;
  copy.height = source.height;
  copy.getContext("2d")?.drawImage(source, 0, 0);
  return copy;
}

function hasPaintedPixels(canvas: HTMLCanvasElement): boolean {
  const pixels = canvas.getContext("2d")?.getImageData(0, 0, canvas.width, canvas.height).data;
  if (!pixels) return false;
  for (let index = 3; index < pixels.length; index += 4) if (pixels[index]) return true;
  return false;
}

function legalDimension(value: number): number {
  return Math.max(64, Math.min(1600, Math.round(value / 64) * 64));
}

function anchorFactors(anchor: string): [number, number] {
  const [vertical, horizontal] = anchor.split("-");
  return [
    horizontal === "left" ? 0 : horizontal === "right" ? 1 : 0.5,
    vertical === "top" ? 0 : vertical === "bottom" ? 1 : 0.5,
  ];
}
