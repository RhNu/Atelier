/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop, jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-tabindex */
import { Check, X } from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppSelect } from "@/components/ui";
import type { ModelCapabilitiesDto, ResourceRefDto } from "@/types";

import { initializePositionDraft } from "../model/character-position";
import type { GenerationCharacterDraft } from "../model/generation-draft";
import { GenerationResourceImage } from "./GenerationResourceImage";

const GRID_COORDS = [0.1, 0.3, 0.5, 0.7, 0.9] as const;
type Guide = "none" | "thirds" | "phi" | "grid";
type Position = { x: number; y: number };

export function CharacterPositionWorkspace({
  characters,
  capabilities,
  size,
  underlayResource,
  underlayStreamSrc,
  onApply,
  onCancel,
}: {
  characters: ReadonlyArray<GenerationCharacterDraft>;
  capabilities: ModelCapabilitiesDto;
  size: { width: number; height: number };
  underlayResource: ResourceRefDto | null;
  underlayStreamSrc: string | null;
  onApply: (characters: GenerationCharacterDraft[]) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("generation");
  const [draft, setDraft] = useState(() => initializePositionDraft(characters));
  const draftRef = useRef(draft);
  const [activeId, setActiveId] = useState(draft[0]?.id ?? "");
  const [guide, setGuide] = useState<Guide>("none");
  const [gridSize, setGridSize] = useState(3);
  const [dirty, setDirty] = useState(false);
  const surfaceRef = useRef<HTMLDivElement>(null);
  const markerRefs = useRef(new Map<string, HTMLButtonElement>());
  const dragRef = useRef<{ id: string; pointerId: number } | null>(null);
  const freeform = capabilities.character_position_mode === "freeform";

  useEffect(() => {
    const currentById = new Map(draftRef.current.map((character) => [character.id, character]));
    const next = initializePositionDraft(characters).map((character) => {
      const current = currentById.get(character.id);
      return current ? { ...character, position: { ...current.position } } : character;
    });
    draftRef.current = next;
    setDraft(next);
    setActiveId((current) =>
      next.some((character) => character.id === current) ? current : (next[0]?.id ?? ""),
    );
  }, [characters]);

  const commitPosition = useCallback((id: string, position: Position) => {
    const rounded = { x: round3(position.x), y: round3(position.y) };
    draftRef.current = draftRef.current.map((item) =>
      item.id === id ? { ...item, position: rounded } : item,
    );
    setDraft(draftRef.current);
    setDirty(true);
  }, []);

  const positionFromPointer = useCallback((event: PointerEvent<HTMLElement>) => {
    const bounds = surfaceRef.current?.getBoundingClientRect();
    if (!bounds) return null;
    return {
      x: clamp01((event.clientX - bounds.left) / bounds.width),
      y: clamp01((event.clientY - bounds.top) / bounds.height),
    };
  }, []);

  function cancel() {
    if (!dirty || window.confirm(t("discardPositionChanges"))) onCancel();
  }

  function handleSurfacePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (!freeform || event.button !== 0) return;
    const position = positionFromPointer(event);
    const active = draftRef.current.find((item) => item.id === activeId);
    if (position && active) commitPosition(active.id, position);
  }

  function handleMarkerPointerDown(event: PointerEvent<HTMLButtonElement>, id: string) {
    event.stopPropagation();
    setActiveId(id);
    if (!freeform || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { id, pointerId: event.pointerId };
  }

  function handleMarkerPointerMove(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const position = positionFromPointer(event);
    const marker = markerRefs.current.get(drag.id);
    if (!position || !marker) return;
    marker.style.left = `${position.x * 100}%`;
    marker.style.top = `${position.y * 100}%`;
    const item = draftRef.current.find((candidate) => candidate.id === drag.id);
    if (item) item.position = { x: round3(position.x), y: round3(position.y) };
    setDirty(true);
  }

  function handleMarkerPointerUp(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    dragRef.current = null;
    setDraft(draftRef.current.map((item) => ({ ...item, position: { ...item.position } })));
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const active = draftRef.current.find((item) => item.id === activeId);
    if (!active) return;
    const step = freeform ? (event.shiftKey ? 0.01 : 0.001) : 0.2;
    const delta =
      event.key === "ArrowLeft"
        ? [-step, 0]
        : event.key === "ArrowRight"
          ? [step, 0]
          : event.key === "ArrowUp"
            ? [0, -step]
            : event.key === "ArrowDown"
              ? [0, step]
              : null;
    if (!delta) return;
    event.preventDefault();
    const next = { x: active.position.x + delta[0], y: active.position.y + delta[1] };
    commitPosition(
      active.id,
      freeform
        ? { x: clamp01(next.x), y: clamp01(next.y) }
        : { x: snapGrid(next.x), y: snapGrid(next.y) },
    );
  }

  const overlapIds = findOverlaps(draft);
  return (
    <section className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)_auto] bg-app-panel">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-app-border px-3 py-2">
        <div>
          <h2 className="text-sm font-semibold text-app-text">{t("characterPositionEditor")}</h2>
          <p className="text-[11px] text-app-muted">{t("characterPositionEditorHint")}</p>
        </div>
        {freeform ? (
          <div className="flex items-center gap-2">
            <AppSelect
              aria-label={t("positionGuide")}
              value={guide}
              options={[
                { value: "none", label: t("guideNone") },
                { value: "thirds", label: t("guideThirds") },
                { value: "phi", label: t("guidePhi") },
                { value: "grid", label: t("guideGrid") },
              ]}
              onValueChange={(value) => setGuide(toGuide(value))}
            />
            {guide === "grid" ? (
              <input
                aria-label={t("gridSize")}
                className="h-8 w-16 border border-app-border bg-app-bg px-2 text-xs text-app-text"
                type="number"
                min={2}
                max={12}
                value={gridSize}
                onChange={(event) =>
                  setGridSize(Math.min(12, Math.max(2, Number(event.target.value) || 3)))
                }
              />
            ) : null}
          </div>
        ) : null}
      </header>
      <div className="grid min-h-0 place-items-center overflow-hidden p-4">
        <div
          ref={surfaceRef}
          role="application"
          aria-label={t("positionCanvas")}
          tabIndex={0}
          className="relative max-h-full max-w-full overflow-hidden border border-app-border bg-white/5 outline-none focus:border-brand-400"
          style={{ aspectRatio: `${size.width} / ${size.height}`, width: "min(100%, 80vh)" }}
          onKeyDown={handleKeyDown}
          onPointerDown={handleSurfacePointerDown}
        >
          {underlayStreamSrc ? (
            <img
              className="absolute inset-0 size-full object-contain opacity-70"
              src={underlayStreamSrc}
              alt=""
            />
          ) : underlayResource ? (
            <GenerationResourceImage
              resource={underlayResource}
              className="absolute inset-0 size-full object-contain opacity-70"
              alt=""
            />
          ) : null}
          {freeform ? (
            <GuideOverlay guide={guide} gridSize={gridSize} />
          ) : (
            <GridOverlay
              onSelect={(position) => {
                if (activeId) commitPosition(activeId, position);
              }}
            />
          )}
          {draft.map((character, index) => (
            <button
              key={character.id}
              ref={(node) => {
                if (node) markerRefs.current.set(character.id, node);
                else markerRefs.current.delete(character.id);
              }}
              type="button"
              aria-label={t("selectCharacter", { index: index + 1 })}
              className={[
                "absolute z-10 min-w-8 -translate-x-1/2 -translate-y-1/2 border px-1.5 py-1 text-[11px] font-bold shadow",
                activeId === character.id
                  ? "border-brand-200 bg-brand-500 text-white"
                  : "border-app-border bg-app-panel text-app-muted",
                overlapIds.has(character.id) ? "ring-2 ring-amber-400" : "",
              ].join(" ")}
              style={{
                left: `${character.position.x * 100}%`,
                top: `${character.position.y * 100}%`,
              }}
              onPointerDown={(event) => handleMarkerPointerDown(event, character.id)}
              onPointerMove={handleMarkerPointerMove}
              onPointerUp={handleMarkerPointerUp}
            >
              {index + 1} · {characterLabel(character)}
            </button>
          ))}
        </div>
      </div>
      <footer className="flex items-center justify-between gap-2 border-t border-app-border px-3 py-2">
        <p className={overlapIds.size ? "text-xs text-amber-300" : "text-xs text-app-muted"}>
          {overlapIds.size ? t("characterPositionOverlap") : t("characterPositionKeyboardHint")}
        </p>
        <div className="flex gap-2">
          <AppButton variant="secondary" onClick={cancel}>
            <X className="size-4" />
            {t("cancel")}
          </AppButton>
          <AppButton onClick={() => onApply(draftRef.current)}>
            <Check className="size-4" />
            {t("applyPositions")}
          </AppButton>
        </div>
      </footer>
    </section>
  );
}

function GridOverlay({ onSelect }: { onSelect: (position: Position) => void }) {
  const { t } = useTranslation("generation");
  return (
    <div className="absolute inset-0 grid grid-cols-5 grid-rows-5">
      {GRID_COORDS.flatMap((y) =>
        GRID_COORDS.map((x) => (
          <button
            key={`${x}-${y}`}
            type="button"
            aria-label={t("positionCell", { x: Math.round(x * 100), y: Math.round(y * 100) })}
            className="border-r border-b border-white/20 hover:bg-brand-500/10"
            onPointerDown={(event) => event.stopPropagation()}
            onClick={() => onSelect({ x, y })}
          />
        )),
      )}
    </div>
  );
}

function GuideOverlay({ guide, gridSize }: { guide: Guide; gridSize: number }) {
  if (guide === "none") return null;
  const lines =
    guide === "thirds"
      ? [1 / 3, 2 / 3]
      : guide === "phi"
        ? [0.382, 0.618]
        : Array.from({ length: gridSize - 1 }, (_, index) => (index + 1) / gridSize);
  return (
    <div className="pointer-events-none absolute inset-0">
      {lines.flatMap((value) => [
        <span
          key={`x-${value}`}
          className="absolute inset-y-0 border-l border-cyan-200/40"
          style={{ left: `${value * 100}%` }}
        />,
        <span
          key={`y-${value}`}
          className="absolute inset-x-0 border-t border-cyan-200/40"
          style={{ top: `${value * 100}%` }}
        />,
      ])}
    </div>
  );
}

function findOverlaps(characters: ReadonlyArray<GenerationCharacterDraft>): Set<string> {
  const overlaps = new Set<string>();
  for (let left = 0; left < characters.length; left += 1)
    for (let right = left + 1; right < characters.length; right += 1) {
      const a = characters[left];
      const b = characters[right];
      if (a && b && Math.hypot(a.position.x - b.position.x, a.position.y - b.position.y) < 0.035) {
        overlaps.add(a.id);
        overlaps.add(b.id);
      }
    }
  return overlaps;
}
function characterLabel(character: GenerationCharacterDraft): string {
  return character.prompt.trim().split(/[,\n]/)[0]?.slice(0, 18) || "Character";
}
function snapGrid(value: number): number {
  return GRID_COORDS.reduce((best, candidate) =>
    Math.abs(candidate - value) < Math.abs(best - value) ? candidate : best,
  );
}
function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}
function round3(value: number): number {
  return Math.round(value * 1000) / 1000;
}
function toGuide(value: string): Guide {
  if (value === "thirds" || value === "phi" || value === "grid") return value;
  return "none";
}
