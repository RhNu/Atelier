import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, EmptyState } from "@/components/ui";
import type { NovelAiExploreQueryDto } from "@/types";

import { NovelAiExploreInspector } from "./components/NovelAiExploreInspector";
import { NovelAiExploreResults } from "./components/NovelAiExploreResults";
import { NovelAiExploreToolbar } from "./components/NovelAiExploreToolbar";
import {
  useExploreTagCompletion,
  useExploreDetailQuery,
  useExploreSearchQuery,
  useExploreSettingsQuery,
} from "./data/useExploreQueries";
import { formatError } from "./explore-utils";
import { useNovelAiExplorePreferences } from "./state/useNovelAiExplorePreferences";

const NO_SUGGESTIONS: never[] = [];

export function NovelAiExploreBrowser({ active }: { active: boolean }) {
  const { t } = useTranslation("explore");
  const [tags, setTags] = useState("");
  const { sort, setSort, period, setPeriod } = useNovelAiExplorePreferences();
  const [creator, setCreator] = useState("");
  const [submitted, setSubmitted] = useState<NovelAiExploreQueryDto | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const search = useExploreSearchQuery(
    submitted ? { source_id: "novelai_explore_gallery", query: submitted } : null,
    active,
  );
  const settings = useExploreSettingsQuery();
  const token = tags.split(",").at(-1)?.trim() ?? "";
  const completion = useExploreTagCompletion(token, active && sort !== "random");
  const items = useMemo(() => {
    const posts =
      search.data?.pages.flatMap((page) =>
        page.items.flatMap((item) =>
          item.source_id === "novelai_explore_gallery" ? [item.post] : [],
        ),
      ) ?? [];
    return [...new Map(posts.map((post) => [post.id, post])).values()];
  }, [search.data]);
  const selectedId = items.some((item) => item.id === selected) ? selected : (items[0]?.id ?? null);
  const detail = useExploreDetailQuery(
    selectedId ? { source_id: "novelai_explore_gallery", item_id: selectedId } : null,
    active,
  );
  const makeQuery = useCallback(
    (creatorId = creator, order = sort): NovelAiExploreQueryDto => ({
      tags:
        order === "random"
          ? []
          : [
              ...new Set(
                tags
                  .split(",")
                  .map((tag) => tag.trim())
                  .filter(Boolean),
              ),
            ],
      sort: order,
      period: order === "new" ? null : period,
      creator_id: order === "random" ? null : creatorId.trim() || null,
      random_salt: order === "random" ? crypto.randomUUID().replaceAll("-", "").slice(0, 6) : null,
    }),
    [creator, sort, tags, period],
  );
  const submit = useCallback(() => {
    const next = makeQuery();
    setSelected(null);
    if (JSON.stringify(next) === JSON.stringify(submitted)) void search.refetch();
    else setSubmitted(next);
  }, [makeQuery, submitted, search]);
  const searchCreator = useCallback(
    (id: string) => {
      const nextSort = sort === "random" ? "new" : sort;
      setCreator(id);
      setSort(nextSort);
      setSelected(null);
      setSubmitted(makeQuery(id, nextSort));
    },
    [sort, makeQuery, setSort],
  );
  const suggestion = useCallback((name: string) => {
    setTags(
      (current) =>
        `${current.slice(0, current.lastIndexOf(",") + 1)}${current.includes(",") ? " " : ""}${name}, `,
    );
  }, []);
  const loadMore = useCallback(() => {
    void search.fetchNextPage();
  }, [search]);
  const retry = useCallback(() => {
    void search.refetch();
  }, [search]);
  const retryDetail = useCallback(() => {
    void detail.refetch();
  }, [detail]);
  const selectedDetail =
    detail.data?.source_id === "novelai_explore_gallery" ? detail.data.detail : undefined;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_390px] divide-x divide-app-border">
        <section className="flex min-h-0 flex-col overflow-hidden">
          <NovelAiExploreToolbar
            tags={tags}
            sort={sort}
            period={period}
            creator={creator}
            searching={search.isFetching && !search.isFetchingNextPage}
            suggestions={completion.data ?? NO_SUGGESTIONS}
            onTags={setTags}
            onSort={setSort}
            onPeriod={setPeriod}
            onCreator={setCreator}
            onSuggestion={suggestion}
            onSubmit={submit}
          />
          <div className="min-h-0 flex-1 overflow-auto p-3">
            {!submitted ? (
              <EmptyState title={t("novelai.startTitle")} />
            ) : search.isPending ? (
              <p className="text-sm text-app-muted">{t("novelai.loading")}</p>
            ) : search.isError && items.length === 0 ? (
              <>
                <EmptyState
                  title={t("novelai.searchFailed")}
                  description={formatError(search.error)}
                />
                <AppButton onClick={retry}>{t("retry")}</AppButton>
              </>
            ) : (
              <NovelAiExploreResults
                items={items}
                selected={selectedId}
                blurSensitive={settings.data?.frontend.gallery.blur_sensitive_images !== false}
                hasNextPage={search.hasNextPage}
                loadingMore={search.isFetchingNextPage}
                error={search.error}
                onSelect={setSelected}
                onLoadMore={loadMore}
              />
            )}
          </div>
        </section>
        <NovelAiExploreInspector
          key={selectedId ?? "empty"}
          detail={selectedDetail}
          pending={selectedId !== null && detail.isPending}
          error={detail.error}
          blurSensitive={settings.data?.frontend.gallery.blur_sensitive_images !== false}
          onCreator={searchCreator}
          onRetry={retryDetail}
        />
      </div>
    </div>
  );
}
