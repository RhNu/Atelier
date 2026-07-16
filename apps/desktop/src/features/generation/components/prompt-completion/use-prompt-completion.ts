import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type MutableRefObject,
  type RefObject,
} from "react";

import { usePromptCompletionQueries } from "@/features/generation/data/usePromptCompletionQueries";
import type { PromptChunkDto, PromptLexiconEntryDto } from "@/types";

import {
  applyPromptCompletion,
  getPromptCompletionContext,
  promptFunctionCompletionItems,
  type PromptCompletionContext,
  type PromptCompletionItem,
} from "./prompt-completion-utils";

type PendingSelection = {
  selectionStart: number;
  selectionEnd: number;
};

type PromptCompletionController = {
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  items: PromptCompletionItem[];
  activeIndex: number;
  manualEmptyPicker: boolean;
  open: boolean;
  acceptItem: (item: PromptCompletionItem) => void;
  handleChange: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  handleClick: (event: MouseEvent<HTMLTextAreaElement>) => void;
  handleBlur: (event: FocusEvent<HTMLTextAreaElement>) => void;
  handleKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
};

const VISIBLE_CHUNK_LIMIT = 12;
const COMPLETION_DEBOUNCE_MS = 120;

export function usePromptCompletionController({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}): PromptCompletionController {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const activeIndexRef = useRef(0);
  const [context, setContext] = useState<PromptCompletionContext | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [pendingSelection, setPendingSelection] = useState<PendingSelection | null>(null);
  const debouncedQuery = useDebouncedValue(context?.query ?? "", COMPLETION_DEBOUNCE_MS);
  const manualEmptyPicker = Boolean(context?.manual && context.query.length === 0);
  const queryResults = usePromptCompletionQueries({ context, debouncedQuery });
  const items = useMemo(
    () =>
      orderCompletionItems({
        context,
        functionItems: promptFunctionCompletionItems(context),
        tagItems: queryResults.tags.map(tagEntryToItem),
        chunkItems: chunkItemsForQuery(queryResults.chunks, context),
      }),
    [context, queryResults.chunks, queryResults.tags],
  );
  const open = Boolean(context) && (items.length > 0 || manualEmptyPicker);

  useEffect(() => {
    if (activeIndex >= items.length) {
      setActiveCompletionIndex(0, activeIndexRef, setActiveIndex);
    }
  }, [activeIndex, items.length]);

  useLayoutEffect(() => {
    if (!pendingSelection) {
      return;
    }

    const textarea = textareaRef.current;
    if (textarea) {
      textarea.focus();
      textarea.setSelectionRange(pendingSelection.selectionStart, pendingSelection.selectionEnd);
    }
    setPendingSelection(null);
  }, [pendingSelection, value]);

  const close = useCallback(() => {
    setContext(null);
    setActiveCompletionIndex(0, activeIndexRef, setActiveIndex);
  }, []);

  const refreshContext = useCallback((textarea: HTMLTextAreaElement, manual = false) => {
    const nextContext = getPromptCompletionContext(textarea.value, textarea.selectionStart, manual);
    const shouldOpen = manual || nextContext.mode !== "tag" || nextContext.query.trim().length > 0;
    setContext(shouldOpen ? nextContext : null);
    setActiveCompletionIndex(0, activeIndexRef, setActiveIndex);
  }, []);

  const acceptItem = useCallback(
    (item: PromptCompletionItem) => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }

      const edit = applyPromptCompletion(value, textarea.selectionStart, item);
      onChange(edit.value);
      setPendingSelection({
        selectionStart: edit.selectionStart,
        selectionEnd: edit.selectionEnd,
      });
      close();
    },
    [close, onChange, value],
  );

  const handleChange = useCallback(
    (event: ChangeEvent<HTMLTextAreaElement>) => {
      onChange(event.target.value);
      refreshContext(event.target);
    },
    [onChange, refreshContext],
  );

  const handleClick = useCallback(
    (event: MouseEvent<HTMLTextAreaElement>) => refreshContext(event.currentTarget),
    [refreshContext],
  );
  const handleBlur = useCallback(
    (event: FocusEvent<HTMLTextAreaElement>) => {
      if (!event.currentTarget.parentElement?.contains(event.relatedTarget as Node | null)) {
        close();
      }
    },
    [close],
  );

  const handleKeyDown = useCompletionKeyDown({
    acceptItem,
    activeIndexRef,
    close,
    items,
    open,
    refreshContext,
    setActiveIndex,
  });

  return {
    textareaRef,
    items,
    activeIndex,
    manualEmptyPicker,
    open,
    acceptItem,
    handleChange,
    handleClick,
    handleBlur,
    handleKeyDown,
  };
}

function useCompletionKeyDown({
  acceptItem,
  activeIndexRef,
  close,
  items,
  open,
  refreshContext,
  setActiveIndex,
}: {
  acceptItem: (item: PromptCompletionItem) => void;
  activeIndexRef: MutableRefObject<number>;
  close: () => void;
  items: PromptCompletionItem[];
  open: boolean;
  refreshContext: (textarea: HTMLTextAreaElement, manual?: boolean) => void;
  setActiveIndex: (index: number) => void;
}) {
  return useCallback(
    (event: KeyboardEvent<HTMLTextAreaElement>) => {
      if ((event.ctrlKey || event.metaKey) && event.key === " ") {
        event.preventDefault();
        refreshContext(event.currentTarget, true);
        return;
      }

      if (!open) {
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        close();
        return;
      }

      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        setActiveCompletionIndex(
          wrapIndex(activeIndexRef.current + step, items.length),
          activeIndexRef,
          setActiveIndex,
        );
        return;
      }

      const activeItem = items[activeIndexRef.current] ?? items[0];
      if ((event.key === "Enter" || event.key === "Tab") && activeItem) {
        event.preventDefault();
        acceptItem(activeItem);
      }
    },
    [acceptItem, activeIndexRef, close, items, open, refreshContext, setActiveIndex],
  );
}

function useDebouncedValue(value: string, delayMs: number): string {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedValue(value);
    }, delayMs);

    return () => window.clearTimeout(timeout);
  }, [delayMs, value]);

  return debouncedValue;
}

function orderCompletionItems({
  context,
  functionItems,
  tagItems,
  chunkItems,
}: {
  context: PromptCompletionContext | null;
  functionItems: PromptCompletionItem[];
  tagItems: PromptCompletionItem[];
  chunkItems: PromptCompletionItem[];
}): PromptCompletionItem[] {
  if (!context) {
    return [];
  }

  if (context.mode === "function") {
    return functionItems;
  }

  if (context.mode === "chunk" || context.manual) {
    return [...chunkItems, ...tagItems];
  }

  return [...tagItems, ...chunkItems];
}

function tagEntryToItem(entry: PromptLexiconEntryDto): PromptCompletionItem {
  return {
    kind: "tag",
    id: `tag:${entry.tag}`,
    label: entry.tag,
    value: entry.tag,
    detail: entry.primary_translation || entry.category,
    rank: entry.match_rank,
  };
}

function chunkItemsForQuery(
  chunks: PromptChunkDto[],
  context: PromptCompletionContext | null,
): PromptCompletionItem[] {
  if (!context) {
    return [];
  }

  const query = normalizeSearchValue(context.query);
  return chunks
    .filter((chunk) => query.length === 0 || chunkMatchesQuery(chunk, query))
    .slice(0, VISIBLE_CHUNK_LIMIT)
    .map((chunk) => ({
      kind: "chunk",
      id: `chunk:${chunk.chunk_id}`,
      label: chunk.key,
      value: chunk.key,
      detail: chunk.description ?? chunk.category ?? chunk.content,
      rank: "workspace",
    }));
}

function chunkMatchesQuery(chunk: PromptChunkDto, query: string): boolean {
  return [chunk.key, chunk.category ?? "", chunk.description ?? "", chunk.content].some((value) =>
    normalizeSearchValue(value).includes(query),
  );
}

function normalizeSearchValue(value: string): string {
  return value.toLowerCase().replaceAll("_", " ").trim();
}

function setActiveCompletionIndex(
  index: number,
  activeIndexRef: MutableRefObject<number>,
  setActiveIndex: (index: number) => void,
) {
  activeIndexRef.current = index;
  setActiveIndex(index);
}

function wrapIndex(index: number, length: number): number {
  if (length === 0) {
    return 0;
  }

  return (index + length) % length;
}
