/* eslint-disable jsx-a11y/prefer-tag-over-role, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { useCallback, type KeyboardEvent } from "react";

import type { GenerationCharacterDraft } from "../model/generation-draft";

const GRID_STEPS = [0, 0.25, 0.5, 0.75, 1] as const;

export function CharacterPositionGrid({
  characters,
  activeIndex,
  onSelectCharacter,
  onChangePosition,
}: {
  characters: ReadonlyArray<GenerationCharacterDraft>;
  activeIndex: number;
  onSelectCharacter: (index: number) => void;
  onChangePosition: (index: number, position: { x: number; y: number }) => void;
}) {
  const active = characters[activeIndex] ?? null;
  const moveActive = useCallback(
    (deltaX: number, deltaY: number) => {
      if (!active) {
        return;
      }
      onChangePosition(activeIndex, {
        x: snap(active.position.x + deltaX),
        y: snap(active.position.y + deltaY),
      });
    },
    [active, activeIndex, onChangePosition],
  );
  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        moveActive(-0.25, 0);
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        moveActive(0.25, 0);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        moveActive(0, -0.25);
      } else if (event.key === "ArrowDown") {
        event.preventDefault();
        moveActive(0, 0.25);
      } else if (event.key === "Home" && active) {
        event.preventDefault();
        onChangePosition(activeIndex, { x: 0.5, y: 0.5 });
      }
    },
    [active, activeIndex, moveActive, onChangePosition],
  );

  return (
    <div className="space-y-2">
      <div
        role="grid"
        aria-label="Character position grid"
        tabIndex={0}
        className="relative grid aspect-[4/3] grid-cols-5 overflow-hidden border border-app-border bg-black/20 outline-none focus:border-brand-400"
        onKeyDown={handleKeyDown}
      >
        {GRID_STEPS.flatMap((y) =>
          GRID_STEPS.map((x) => (
            <button
              key={`${x}-${y}`}
              type="button"
              role="gridcell"
              aria-label={`Position ${Math.round(x * 100)}%, ${Math.round(y * 100)}%`}
              className="border-r border-b border-app-border/60 last:border-r-0 hover:bg-brand-500/10"
              onClick={() => active && onChangePosition(activeIndex, { x, y })}
            />
          )),
        )}
        {characters.map((character, index) => (
          <button
            key={character.id}
            type="button"
            aria-label={`Select character ${index + 1}`}
            className={[
              "absolute grid size-6 -translate-x-1/2 -translate-y-1/2 place-items-center border text-[11px] font-bold shadow",
              activeIndex === index
                ? "z-20 border-brand-300 bg-brand-500 text-white"
                : "z-10 border-app-border bg-app-panel text-app-muted",
            ].join(" ")}
            style={{
              left: `${character.position.x * 100}%`,
              top: `${character.position.y * 100}%`,
            }}
            onClick={() => onSelectCharacter(index)}
          >
            {index + 1}
          </button>
        ))}
      </div>
      <p className="text-[11px] text-app-muted">
        Select a character marker, then click a cell or use arrow keys. Home centers it.
      </p>
    </div>
  );
}

function snap(value: number): number {
  return Math.min(1, Math.max(0, Math.round(value * 4) / 4));
}
