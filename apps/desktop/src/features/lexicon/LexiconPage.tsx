import { useNavigate } from "@tanstack/react-router";
import {
  startTransition,
  type KeyboardEvent,
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type {
  LexiconCategoryDto,
  LexiconEntityKindDto,
  LexiconSearchItemDto,
  LexiconSearchModeDto,
  LexiconSearchRequestDto,
} from "@/types";

import { LexiconBasket } from "./components/LexiconBasket";
import { LexiconFilters, type LexiconRatingFilter } from "./components/LexiconFilters";
import { LexiconInspector } from "./components/LexiconInspector";
import { LexiconResults } from "./components/LexiconResults";
import {
  useAppendLexiconEntitiesMutation,
  useLexiconBootstrapQuery,
  useLexiconEntityQuery,
  useLexiconSearchQuery,
} from "./data/useLexiconQueries";

const SEARCH_LIMIT = 100;
const RATING_STORAGE_KEY = "atelier.lexicon.rating.v1";

export function LexiconPage() {
  const { t } = useTranslation("lexicon");
  const navigate = useNavigate();
  const pushToast = useToastStore((state) => state.push);
  const [mode, setMode] = useState<LexiconSearchModeDto>("lexical");
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<"all" | LexiconEntityKindDto>("all");
  const [category, setCategory] = useState<"all" | LexiconCategoryDto>("all");
  const [groupId, setGroupId] = useState("");
  const [rating, setRating] = useState<LexiconRatingFilter>(readRatingPreference);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [basket, setBasket] = useState<Map<number, LexiconSearchItemDto>>(() => new Map());
  const inspectorRef = useRef<HTMLDialogElement>(null);
  const deferredQuery = useDeferredValue(query.trim());
  const bootstrap = useLexiconBootstrapQuery();
  const request = useMemo<LexiconSearchRequestDto>(
    () => buildSearchRequest({ deferredQuery, mode, kind, category, groupId, rating, basket }),
    [basket, category, deferredQuery, groupId, kind, mode, rating],
  );
  const results = useLexiconSearchQuery(request);
  const detail = useLexiconEntityQuery(selectedId);
  const append = useAppendLexiconEntitiesMutation();
  const basketIds = useMemo(() => new Set(basket.keys()), [basket]);
  const basketItems = useMemo(() => [...basket.values()], [basket]);
  useEffect(() => {
    if (selectedId !== null) inspectorRef.current?.focus();
  }, [selectedId]);

  const toggleBasket = useCallback((item: LexiconSearchItemDto) => {
    setBasket((current) => {
      const next = new Map(current);
      if (next.has(item.entity_id)) next.delete(item.entity_id);
      else next.set(item.entity_id, item);
      return next;
    });
  }, []);
  const handleRatingChange = useCallback((value: LexiconRatingFilter) => {
    setRating(value);
    localStorage.setItem(RATING_STORAGE_KEY, value);
  }, []);
  const handleModeChange = useCallback((value: LexiconSearchModeDto) => {
    startTransition(() => setMode(value));
  }, []);
  const closeInspector = useCallback(() => setSelectedId(null), []);
  const handleInspectorKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDialogElement>) => {
      if (event.key === "Escape") closeInspector();
    },
    [closeInspector],
  );
  const removeBasketItem = useCallback((entityId: number) => {
    setBasket((current) => {
      const next = new Map(current);
      next.delete(entityId);
      return next;
    });
  }, []);
  const clearBasket = useCallback(() => setBasket(new Map()), []);
  const submit = useCallback(
    (target: "positive" | "negative") => {
      void append
        .mutateAsync({ target, entity_ids: [...basket.keys()] })
        .then(async () => {
          setBasket(new Map());
          pushToast({ level: "success", message: t("addedToDraft") });
          await navigate({ to: "/generate" });
        })
        .catch((error: unknown) => {
          pushToast({ level: "error", message: formatError(error) });
        });
    },
    [append, basket, navigate, pushToast, t],
  );

  if (bootstrap.isPending) {
    return <p className="p-4 text-sm text-app-muted">{t("loadingCatalog")}</p>;
  }
  if (bootstrap.isError || !bootstrap.data?.status.lexical_available) {
    return (
      <EmptyState
        title={t("unavailable")}
        description={
          bootstrap.data?.status.message ??
          (bootstrap.error ? formatError(bootstrap.error) : undefined)
        }
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <LexiconFilters
        bootstrap={bootstrap.data}
        mode={mode}
        query={query}
        kind={kind}
        category={category}
        groupId={groupId}
        rating={rating}
        onModeChange={handleModeChange}
        onQueryChange={setQuery}
        onKindChange={setKind}
        onCategoryChange={setCategory}
        onGroupChange={setGroupId}
        onRatingChange={handleRatingChange}
      />
      <div className="relative grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] max-xl:grid-cols-1">
        <LexiconResults
          page={results.data}
          pending={results.isPending}
          error={results.error ? formatError(results.error) : null}
          selectedId={selectedId}
          basketIds={basketIds}
          onInspect={setSelectedId}
          onToggleBasket={toggleBasket}
        />
        <dialog
          open
          ref={inspectorRef}
          tabIndex={-1}
          aria-label={t("details")}
          onKeyDown={handleInspectorKeyDown}
          className={[
            "min-h-0 max-xl:absolute max-xl:inset-y-0 max-xl:right-0 max-xl:z-20 max-xl:w-[min(420px,90vw)] max-xl:shadow-2xl",
            selectedId === null ? "max-xl:hidden" : "",
          ].join(" ")}
        >
          <LexiconInspector
            detail={detail.data}
            pending={detail.isPending}
            error={detail.error ? formatError(detail.error) : null}
            inBasket={detail.data ? basket.has(detail.data.entity.entity_id) : false}
            onClose={closeInspector}
            onToggleBasket={toggleBasket}
            onInspectRelated={setSelectedId}
          />
        </dialog>
      </div>
      <LexiconBasket
        items={basketItems}
        pending={append.isPending}
        onRemove={removeBasketItem}
        onClear={clearBasket}
        onSubmit={submit}
      />
    </div>
  );
}

function buildSearchRequest({
  deferredQuery,
  mode,
  kind,
  category,
  groupId,
  rating,
  basket,
}: {
  deferredQuery: string;
  mode: LexiconSearchModeDto;
  kind: "all" | LexiconEntityKindDto;
  category: "all" | LexiconCategoryDto;
  groupId: string;
  rating: LexiconRatingFilter;
  basket: Map<number, LexiconSearchItemDto>;
}): LexiconSearchRequestDto {
  return {
    query: deferredQuery,
    mode,
    filters: {
      entity_kinds: kind === "all" ? [] : [kind],
      categories: category === "all" ? [] : [category],
      group_ids: groupId ? [groupId] : [],
      ratings: rating === "all" ? [] : [rating],
    },
    selected_entity_ids: [...basket.keys()],
    offset: 0,
    limit: SEARCH_LIMIT,
  };
}

function readRatingPreference(): LexiconRatingFilter {
  const value = localStorage.getItem(RATING_STORAGE_KEY);
  return value === "safe" || value === "sensitive" || value === "unknown" ? value : "all";
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
