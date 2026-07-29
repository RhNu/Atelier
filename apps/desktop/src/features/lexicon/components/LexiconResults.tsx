import { useVirtualizer } from "@tanstack/react-virtual";
import { Check, Plus } from "lucide-react";
import { type CSSProperties, type KeyboardEvent, useCallback, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, EmptyState } from "@/components/ui";
import type { LexiconSearchItemDto, LexiconSearchPageDto } from "@/types";

type Props = {
  page: LexiconSearchPageDto | undefined;
  pending: boolean;
  error: string | null;
  selectedId: number | null;
  basketIds: Set<number>;
  onInspect: (entityId: number) => void;
  onToggleBasket: (item: LexiconSearchItemDto) => void;
};

type RowPosition = {
  index: number;
  start: number;
  size: number;
};

const ROW_HEIGHT = 72;

export function LexiconResults({
  page,
  pending,
  error,
  selectedId,
  basketIds,
  onInspect,
  onToggleBasket,
}: Props) {
  const { t } = useTranslation("lexicon");
  const scrollRef = useRef<HTMLDivElement>(null);
  const items = page?.items ?? [];
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
    initialRect: { width: 800, height: 600 },
  });
  const measuredRows = virtualizer.getVirtualItems();
  const visibleRows =
    measuredRows.length > 0
      ? measuredRows
      : items.slice(0, 12).map((_, index) => ({
          index,
          start: index * ROW_HEIGHT,
          size: ROW_HEIGHT,
        }));
  const listStyle = useMemo<CSSProperties>(
    () => ({
      height: `${Math.max(virtualizer.getTotalSize(), items.length * ROW_HEIGHT)}px`,
    }),
    [items.length, virtualizer],
  );
  const moveFocus = useCallback(
    (currentIndex: number, delta: number) => {
      const next = Math.max(0, Math.min(items.length - 1, currentIndex + delta));
      virtualizer.scrollToIndex(next);
      requestAnimationFrame(() => {
        scrollRef.current?.querySelector<HTMLElement>(`[data-lexicon-index="${next}"]`)?.focus();
      });
    },
    [items.length, virtualizer],
  );
  if (pending && !page) {
    return <p className="p-4 text-sm text-app-muted">{t("loadingEntries")}</p>;
  }
  if (error) return <EmptyState title={t("queryFailed")} description={error} />;
  if (!page || page.items.length === 0) return <EmptyState title={t("noMatchingTags")} />;

  return (
    <div ref={scrollRef} className="min-h-0 overflow-auto">
      <ul className="relative w-full" style={listStyle} aria-label={t("searchResults")}>
        {visibleRows.map((virtualItem) => {
          const item = items[virtualItem.index];
          if (!item) return null;
          return (
            <LexiconResultRow
              key={item.entity_id}
              position={virtualItem}
              item={item}
              selected={selectedId === item.entity_id}
              inBasket={basketIds.has(item.entity_id)}
              onInspect={onInspect}
              onToggleBasket={onToggleBasket}
              onMoveFocus={moveFocus}
            />
          );
        })}
      </ul>
    </div>
  );
}

function LexiconResultRow({
  position,
  item,
  selected,
  inBasket,
  onInspect,
  onToggleBasket,
  onMoveFocus,
}: {
  position: RowPosition;
  item: LexiconSearchItemDto;
  selected: boolean;
  inBasket: boolean;
  onInspect: (entityId: number) => void;
  onToggleBasket: (item: LexiconSearchItemDto) => void;
  onMoveFocus: (currentIndex: number, delta: number) => void;
}) {
  const { t } = useTranslation("lexicon");
  const style = useMemo<CSSProperties>(
    () => ({
      position: "absolute",
      insetInline: 0,
      top: 0,
      height: `${position.size}px`,
      transform: `translateY(${position.start}px)`,
    }),
    [position.size, position.start],
  );
  const inspect = useCallback(() => onInspect(item.entity_id), [item.entity_id, onInspect]);
  const toggle = useCallback(() => onToggleBasket(item), [item, onToggleBasket]);
  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLButtonElement>) => {
      if (event.target !== event.currentTarget) return;
      if (event.key === "Enter") {
        event.preventDefault();
        inspect();
      } else if (event.key === " ") {
        event.preventDefault();
        toggle();
      } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        onMoveFocus(position.index, event.key === "ArrowDown" ? 1 : -1);
      }
    },
    [inspect, onMoveFocus, position.index, toggle],
  );
  return (
    <li
      aria-current={selected ? "true" : undefined}
      style={style}
      className={[
        "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-b border-app-border px-4 py-2",
        selected ? "bg-brand-500/10" : "hover:bg-white/[0.025]",
      ].join(" ")}
    >
      <button
        type="button"
        className="min-w-0 text-left"
        data-lexicon-index={position.index}
        onClick={inspect}
        onKeyDown={handleKeyDown}
      >
        <div className="flex items-center gap-2">
          <code className="truncate text-sm font-semibold text-brand-100">
            {item.canonical_name}
          </code>
          <Badge value={item.kind} />
          <Badge value={item.category} />
          {item.rating === "sensitive" ? (
            <span className="border border-rose-500/40 bg-rose-500/10 px-1 text-[9px] text-rose-200 uppercase">
              NSFW
            </span>
          ) : null}
        </div>
        <p className="mt-1 truncate text-sm text-app-text">
          {item.primary_translation || item.canonical_name}
        </p>
        <p className="mt-1 truncate text-[10px] text-app-muted">
          {t("matched", { value: item.matched_text })} · {item.match_reason} ·{" "}
          {t("posts", { value: item.post_count.toLocaleString() })}
        </p>
      </button>
      <AppButton
        variant={inBasket ? "secondary" : "ghost"}
        className="size-8 p-0"
        aria-label={inBasket ? t("removeFromBasket") : t("addToBasket")}
        onClick={toggle}
      >
        {inBasket ? (
          <Check aria-hidden="true" className="size-4" />
        ) : (
          <Plus aria-hidden="true" className="size-4" />
        )}
      </AppButton>
    </li>
  );
}

function Badge({ value }: { value: string }) {
  return (
    <span className="border border-app-border px-1 text-[9px] text-app-muted uppercase">
      {value}
    </span>
  );
}
