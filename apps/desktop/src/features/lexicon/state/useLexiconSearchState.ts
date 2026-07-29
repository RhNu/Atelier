import { startTransition, useCallback, useDeferredValue, useMemo, useState } from "react";

import type {
  LexiconCategoryDto,
  LexiconEntityKindDto,
  LexiconSearchItemDto,
  LexiconSearchModeDto,
  LexiconSearchRequestDto,
} from "@/types";

import type { LexiconRatingFilter } from "../components/LexiconFilters";
import { useLexiconSearchQuery } from "../data/useLexiconQueries";

const SEARCH_LIMIT = 100;
const RATING_STORAGE_KEY = "atelier.lexicon.rating.v1";

export function useLexiconSearchState(basket: Map<number, LexiconSearchItemDto>) {
  const [mode, setMode] = useState<LexiconSearchModeDto>("lexical");
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<"all" | LexiconEntityKindDto>("all");
  const [category, setCategory] = useState<"all" | LexiconCategoryDto>("all");
  const [groupId, setGroupId] = useState("");
  const [rating, setRating] = useState<LexiconRatingFilter>(readRatingPreference);
  const [offset, setOffset] = useState(0);
  const [semanticQuery, setSemanticQuery] = useState("");
  const [semanticContextIds, setSemanticContextIds] = useState<number[]>([]);
  const deferredQuery = useDeferredValue(query.trim());
  const activeQuery = mode === "semantic" ? semanticQuery : deferredQuery;
  const request = useMemo<LexiconSearchRequestDto>(
    () =>
      buildSearchRequest({
        query: activeQuery,
        mode,
        kind,
        category,
        groupId,
        rating,
        selectedEntityIds: mode === "semantic" ? semanticContextIds : [],
        offset,
      }),
    [activeQuery, category, groupId, kind, mode, offset, rating, semanticContextIds],
  );
  const results = useLexiconSearchQuery(request);
  const semanticBusy = mode === "semantic" && results.fetchStatus === "fetching";
  const hasActiveSearch =
    query.trim().length > 0 ||
    mode !== "lexical" ||
    kind !== "all" ||
    category !== "all" ||
    groupId !== "" ||
    rating !== "all";
  const changeMode = useCallback((value: LexiconSearchModeDto) => {
    startTransition(() => {
      setMode(value);
      setOffset(0);
      setSemanticQuery("");
      setSemanticContextIds([]);
    });
  }, []);
  const changeQuery = useCallback((value: string) => {
    setQuery(value);
    setOffset(0);
  }, []);
  const clearQuery = useCallback(() => {
    setQuery("");
    setSemanticQuery("");
    setSemanticContextIds([]);
    setOffset(0);
  }, []);
  const changeKind = useCallback((value: "all" | LexiconEntityKindDto) => {
    setKind(value);
    setOffset(0);
  }, []);
  const changeCategory = useCallback((value: "all" | LexiconCategoryDto) => {
    setCategory(value);
    setOffset(0);
  }, []);
  const changeGroup = useCallback((value: string) => {
    setGroupId(value);
    setOffset(0);
  }, []);
  const changeRating = useCallback((value: LexiconRatingFilter) => {
    setRating(value);
    setOffset(0);
    localStorage.setItem(RATING_STORAGE_KEY, value);
  }, []);
  const reset = useCallback(() => {
    startTransition(() => {
      setMode("lexical");
      setQuery("");
      setKind("all");
      setCategory("all");
      setGroupId("");
      setRating("all");
      setSemanticQuery("");
      setSemanticContextIds([]);
      setOffset(0);
    });
    localStorage.setItem(RATING_STORAGE_KEY, "all");
  }, []);
  const submitSemantic = useCallback(() => {
    const nextQuery = query.trim();
    if (!nextQuery) return;
    const nextContextIds = [...basket.keys()];
    const contextUnchanged =
      nextContextIds.length === semanticContextIds.length &&
      nextContextIds.every((entityId, index) => entityId === semanticContextIds[index]);
    if (nextQuery === semanticQuery && contextUnchanged) {
      void results.refetch();
      return;
    }
    setSemanticContextIds(nextContextIds);
    setSemanticQuery(nextQuery);
    setOffset(0);
  }, [basket, query, results, semanticContextIds, semanticQuery]);
  return {
    mode,
    query,
    kind,
    category,
    groupId,
    rating,
    semanticQuery,
    results,
    semanticBusy,
    hasActiveSearch,
    changeMode,
    changeQuery,
    clearQuery,
    changeKind,
    changeCategory,
    changeGroup,
    changeRating,
    reset,
    submitSemantic,
    changeOffset: setOffset,
  };
}

function buildSearchRequest({
  query,
  mode,
  kind,
  category,
  groupId,
  rating,
  selectedEntityIds,
  offset,
}: {
  query: string;
  mode: LexiconSearchModeDto;
  kind: "all" | LexiconEntityKindDto;
  category: "all" | LexiconCategoryDto;
  groupId: string;
  rating: LexiconRatingFilter;
  selectedEntityIds: number[];
  offset: number;
}): LexiconSearchRequestDto {
  return {
    query,
    mode,
    filters: {
      entity_kinds: kind === "all" ? [] : [kind],
      categories: category === "all" ? [] : [category],
      group_ids: groupId ? [groupId] : [],
      ratings: rating === "all" ? [] : [rating],
    },
    selected_entity_ids: selectedEntityIds,
    offset,
    limit: SEARCH_LIMIT,
  };
}

function readRatingPreference(): LexiconRatingFilter {
  const value = localStorage.getItem(RATING_STORAGE_KEY);
  return value === "safe" || value === "sensitive" || value === "unknown" ? value : "all";
}
