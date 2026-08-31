import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { useTranslation } from "react-i18next";

import { runLoggedAction } from "@/app/logger";
import { AppPanel } from "@/components/ui";
import { desktopApi } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type { DanbooruExploreQueryDto, DanbooruTagDto } from "@/types";

import { DanbooruInspector } from "./components/DanbooruInspector";
import { DanbooruResults } from "./components/DanbooruResults";
import { DanbooruSearchToolbar } from "./components/DanbooruSearchToolbar";
import { DanbooruTagBasket } from "./components/DanbooruTagBasket";
import { useDanbooruDetailQuery, useDanbooruSearchQuery } from "./data/useDanbooruQueries";
import { useExploreTagCompletion } from "./data/useExploreQueries";
import { useCopyExploreText, useExploreSettingsQuery } from "./data/useExploreQueries";
import {
  appendSearchTag,
  currentTagToken,
  formatPromptTags,
  formatQueryTags,
  hasRatingMetatag,
  orderSelectedTags,
  replaceCurrentToken,
  selectedRatings,
} from "./explore-utils";

type SearchInput = DanbooruExploreQueryDto;
const NO_SUGGESTIONS: never[] = [];
const SHOW_ADULT_STORAGE_KEY = "atelier.explore.danbooru.show-adult.v1";

export function DanbooruBrowser({ active }: { active: boolean }) {
  const { t } = useTranslation("explore");
  const pushToast = useToastStore((state) => state.push);
  const [draftQuery, setDraftQuery] = useState("");
  const [showAdult, setShowAdult] = useState(readShowAdultPreference);
  const [submitted, setSubmitted] = useState<SearchInput | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectedTags, setSelectedTags] = useState<Map<string, DanbooruTagDto>>(() => new Map());
  const validationError = hasRatingMetatag(draftQuery) ? t("ratingConflict") : null;
  const token = currentTagToken(draftQuery);
  const completion = useExploreTagCompletion(token, active);
  const search = useDanbooruSearchQuery(submitted, active);
  const detail = useDanbooruDetailQuery(selectedId, active);
  const settings = useExploreSettingsQuery();
  const items = useMemo(
    () => search.data?.pages.flatMap((page) => page.items) ?? [],
    [search.data?.pages],
  );
  const orderedTags = useMemo(() => orderSelectedTags(selectedTags.values()), [selectedTags]);
  const selectedTagNames = useMemo(() => new Set(selectedTags.keys()), [selectedTags]);
  const blurSensitive = settings.data?.frontend.gallery.blur_sensitive_images !== false;

  useFirstResultSelection(items, setSelectedId);

  const submitSearch = useCallback(() => {
    if (validationError) return;
    setSubmitted({
      query: draftQuery.trim(),
      ratings: selectedRatings(showAdult),
    });
    setSelectedId(null);
  }, [draftQuery, showAdult, validationError]);

  const changeShowAdult = useCallback((value: boolean) => {
    setShowAdult(value);
    window.localStorage.setItem(SHOW_ADULT_STORAGE_KEY, String(value));
  }, []);

  const chooseSuggestion = useCallback((canonicalName: string) => {
    setDraftQuery((current) => replaceCurrentToken(current, canonicalName));
  }, []);

  const searchTag = useCallback(
    (tag: string) => {
      const next = appendSearchTag(draftQuery, tag);
      setDraftQuery(next);
      setSubmitted({ query: next, ratings: selectedRatings(showAdult) });
      setSelectedId(null);
    },
    [draftQuery, showAdult],
  );

  const toggleTag = useCallback((tag: DanbooruTagDto) => {
    setSelectedTags((current) => {
      const next = new Map(current);
      if (next.has(tag.canonical_name)) next.delete(tag.canonical_name);
      else next.set(tag.canonical_name, tag);
      return next;
    });
  }, []);

  const copy = useCopyExploreText();

  const openExternal = useCallback(
    (url: string) => {
      void runLoggedAction("Open Danbooru external URL", () =>
        desktopApi.openExternalUrl(url),
      ).catch((error: unknown) =>
        pushToast({ level: "error", title: t("openFailed"), message: String(error) }),
      );
    },
    [pushToast, t],
  );

  const removeTag = useCallback((name: string) => {
    setSelectedTags((current) => {
      const next = new Map(current);
      next.delete(name);
      return next;
    });
  }, []);
  const loadMore = useCallback(() => {
    void search.fetchNextPage();
  }, [search]);
  const retrySearch = useCallback(() => {
    void search.refetch();
  }, [search]);
  const clearTags = useCallback(() => setSelectedTags(new Map()), []);
  const copyPrompt = useCallback(
    () => copy.mutate(formatPromptTags(orderedTags)),
    [copy, orderedTags],
  );
  const copyQuery = useCallback(
    () => copy.mutate(formatQueryTags(orderedTags)),
    [copy, orderedTags],
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_390px] divide-x divide-app-border">
        <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
          <DanbooruSearchToolbar
            query={draftQuery}
            showAdult={showAdult}
            suggestions={completion.data ?? NO_SUGGESTIONS}
            validationError={validationError}
            searching={search.isFetching && !search.isFetchingNextPage}
            onQueryChange={setDraftQuery}
            onAdultChange={changeShowAdult}
            onSuggestion={chooseSuggestion}
            onSubmit={submitSearch}
          />
          <div className="min-h-0 flex-1 overflow-auto p-3">
            <DanbooruResults
              items={items}
              selectedId={selectedId}
              blurSensitive={blurSensitive}
              pending={search.isPending}
              error={search.error}
              searched={submitted !== null}
              hasNextPage={search.hasNextPage}
              loadingMore={search.isFetchingNextPage}
              onSelect={setSelectedId}
              onLoadMore={loadMore}
              onRetry={retrySearch}
            />
          </div>
        </AppPanel>
        <DanbooruInspector
          detail={detail.data}
          pending={selectedId !== null && detail.isPending}
          error={detail.error}
          selectedTags={selectedTagNames}
          blurSensitive={blurSensitive}
          onToggleTag={toggleTag}
          onSearchTag={searchTag}
          onOpenExternal={openExternal}
        />
      </div>
      <DanbooruTagBasket
        tags={orderedTags}
        copying={copy.isPending}
        onRemove={removeTag}
        onClear={clearTags}
        onCopyPrompt={copyPrompt}
        onCopyQuery={copyQuery}
      />
    </div>
  );
}

function useFirstResultSelection(
  items: { id: number }[],
  setSelectedId: Dispatch<SetStateAction<number | null>>,
) {
  useEffect(() => {
    setSelectedId((current) => {
      if (current !== null && items.some((item) => item.id === current)) return current;
      return items[0]?.id ?? null;
    });
  }, [items, setSelectedId]);
}

function readShowAdultPreference(): boolean {
  return (
    typeof window !== "undefined" &&
    (window.localStorage.getItem(SHOW_ADULT_STORAGE_KEY) ??
      window.localStorage.getItem("atelier.inspiration.show-adult.v1")) === "true"
  );
}
