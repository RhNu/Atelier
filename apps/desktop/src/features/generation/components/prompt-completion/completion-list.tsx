/* eslint-disable jsx-a11y/prefer-tag-over-role */
import { useCallback, type MouseEvent } from "react";

import type { PromptCompletionItem } from "./prompt-completion-utils";

type CompletionListProps = {
  id: string;
  items: PromptCompletionItem[];
  activeIndex: number;
  optionIdPrefix: string;
  manualEmptyPicker: boolean;
  onAccept: (item: PromptCompletionItem) => void;
};

export function CompletionList({
  id,
  items,
  activeIndex,
  optionIdPrefix,
  manualEmptyPicker,
  onAccept,
}: CompletionListProps) {
  return (
    <div
      id={id}
      role="listbox"
      aria-label="Prompt completions"
      className="absolute z-30 mt-1 max-h-72 w-full overflow-auto border border-app-border bg-app-panel shadow-app-panel"
    >
      {manualEmptyPicker ? (
        <p className="border-b border-app-border px-3 py-2 text-xs text-app-muted">
          Type to search NovelAI tags. Prompt chunks are available now.
        </p>
      ) : null}
      {items.map((item, index) => (
        <CompletionOption
          key={item.id}
          id={`${optionIdPrefix}-option-${index}`}
          item={item}
          active={index === activeIndex}
          onAccept={onAccept}
        />
      ))}
    </div>
  );
}

function CompletionOption({
  id,
  item,
  active,
  onAccept,
}: {
  id: string;
  item: PromptCompletionItem;
  active: boolean;
  onAccept: (item: PromptCompletionItem) => void;
}) {
  const handleMouseDown = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      event.preventDefault();
      onAccept(item);
    },
    [item, onAccept],
  );

  return (
    <button
      id={id}
      type="button"
      role="option"
      aria-selected={active}
      className={completionOptionClassName(active)}
      onMouseDown={handleMouseDown}
    >
      <span className="min-w-0">
        <span className="block truncate font-semibold">{item.label}</span>
        {item.detail ? (
          <span className="block truncate text-xs text-app-muted">{item.detail}</span>
        ) : null}
      </span>
      <span className="text-xs text-app-muted uppercase">{item.kind}</span>
    </button>
  );
}

function completionOptionClassName(active: boolean): string {
  return [
    "grid w-full grid-cols-[minmax(0,1fr)_auto] gap-2 border-b border-app-border px-3 py-2 text-left text-sm last:border-b-0",
    active ? "bg-brand-500/20 text-brand-100" : "bg-app-panel text-app-text hover:bg-app-surface",
  ].join(" ");
}
