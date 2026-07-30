import { UserRound } from "lucide-react";
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
import type { DanbooruSearchRequestDto, DanbooruTagDto } from "@/types";

import { DanbooruInspector } from "./components/DanbooruInspector";
import { DanbooruResults } from "./components/DanbooruResults";
import { DanbooruTagBasket } from "./components/DanbooruTagBasket";
import { InspirationSearchToolbar } from "./components/InspirationSearchToolbar";
import {
  useDanbooruAccountQuery,
  useCopyDanbooruTagsMutation,
  useDanbooruDetailQuery,
  useDanbooruGlobalSettingsQuery,
  useDanbooruSearchQuery,
  useDanbooruTagCompletion,
} from "./data/useDanbooruQueries";
import {
  appendSearchTag,
  currentTagToken,
  formatPromptTags,
  formatQueryTags,
  hasRatingMetatag,
  orderSelectedTags,
  replaceCurrentToken,
  selectedRatings,
} from "./inspiration-utils";

type SearchInput = Omit<DanbooruSearchRequestDto, "before_id">;
const NO_SUGGESTIONS: never[] = [];

export function InspirationPage() {
  const { t } = useTranslation("inspiration");
  const pushToast = useToastStore((state) => state.push);
  const [draftQuery, setDraftQuery] = useState("");
  const [showAdult, setShowAdult] = useState(false);
  const [submitted, setSubmitted] = useState<SearchInput | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectedTags, setSelectedTags] = useState<Map<string, DanbooruTagDto>>(() => new Map());
  const validationError = hasRatingMetatag(draftQuery) ? t("ratingConflict") : null;
  const token = currentTagToken(draftQuery);
  const completion = useDanbooruTagCompletion(token);
  const search = useDanbooruSearchQuery(submitted);
  const detail = useDanbooruDetailQuery(selectedId);
  const account = useDanbooruAccountQuery();
  const settings = useDanbooruGlobalSettingsQuery();
  const items = useMemo(
    () => search.data?.pages.flatMap((page) => page.items) ?? [],
    [search.data?.pages],
  );
  const orderedTags = useMemo(() => orderSelectedTags(selectedTags.values()), [selectedTags]);
  const selectedTagNames = useMemo(() => new Set(selectedTags.keys()), [selectedTags]);
  const blurSensitive =
    settings.data?.frontend.gallery.blur_sensitive_images === true && !settings.isError;

  useFirstResultSelection(items, setSelectedId);

  const submitSearch = useCallback(() => {
    if (validationError) return;
    setSubmitted({
      query: draftQuery.trim(),
      ratings: selectedRatings(showAdult),
    });
    setSelectedId(null);
  }, [draftQuery, showAdult, validationError]);

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

  const copy = useCopyDanbooruTagsMutation();

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
      <InspirationHeader
        accountName={
          account.data?.configured
            ? (account.data.username ?? t("configuredAccount"))
            : t("anonymousMode")
        }
      />
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_390px] divide-x divide-app-border">
        <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
          <InspirationSearchToolbar
            query={draftQuery}
            showAdult={showAdult}
            suggestions={completion.data ?? NO_SUGGESTIONS}
            validationError={validationError}
            searching={search.isFetching && !search.isFetchingNextPage}
            onQueryChange={setDraftQuery}
            onAdultChange={setShowAdult}
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

function InspirationHeader({ accountName }: { accountName: string }) {
  const { t } = useTranslation("inspiration");
  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-app-border bg-app-panel px-3">
      <div>
        <h1 className="text-sm font-semibold text-white">{t("title")}</h1>
        <p className="text-[11px] text-app-muted">{t("subtitle")}</p>
      </div>
      <div className="flex items-center gap-2 text-xs text-app-muted">
        <UserRound aria-hidden="true" className="size-4" />
        {accountName}
      </div>
    </header>
  );
}
