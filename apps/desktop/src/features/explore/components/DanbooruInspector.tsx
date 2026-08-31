import { ExternalLink, Search } from "lucide-react";
import { useCallback, useMemo, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, EmptyState } from "@/components/ui";
import type { DanbooruPostDetailDto, DanbooruTagCategoryDto, DanbooruTagDto } from "@/types";

import { formatBytes, formatError, shouldBlurRating } from "../explore-utils";
import { DanbooruImage } from "./DanbooruImage";

const CATEGORIES: DanbooruTagCategoryDto[] = [
  "artist",
  "copyright",
  "character",
  "general",
  "meta",
];

type Props = {
  detail: DanbooruPostDetailDto | undefined;
  pending: boolean;
  error: unknown;
  selectedTags: ReadonlySet<string>;
  blurSensitive: boolean;
  onToggleTag: (tag: DanbooruTagDto) => void;
  onSearchTag: (tag: string) => void;
  onOpenExternal: (url: string) => void;
};

export function DanbooruInspector({
  detail,
  pending,
  error,
  selectedTags,
  blurSensitive,
  onToggleTag,
  onSearchTag,
  onOpenExternal,
}: Props) {
  const { t } = useTranslation("explore");
  const groupedTags = useMemo(
    () =>
      CATEGORIES.map((category) => ({
        category,
        tags: detail?.tags.filter((tag) => tag.category === category) ?? [],
      })),
    [detail],
  );
  const openDanbooru = useCallback(() => {
    if (detail) onOpenExternal(detail.danbooru_url);
  }, [detail, onOpenExternal]);
  const openSource = useCallback(() => {
    if (detail?.source_url) onOpenExternal(detail.source_url);
  }, [detail, onOpenExternal]);
  if (pending) {
    return (
      <AppPanel variant="section" className="p-3 text-sm text-app-muted">
        {t("loadingDetail")}
      </AppPanel>
    );
  }
  if (error) {
    return (
      <AppPanel variant="section">
        <EmptyState title={t("detailFailed")} description={formatError(error)} />
      </AppPanel>
    );
  }
  if (!detail) {
    return (
      <AppPanel variant="section">
        <EmptyState title={t("selectDetail")} />
      </AppPanel>
    );
  }
  const sourceUrl = detail.source_url;

  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex items-center justify-between border-b border-app-border px-3 py-2">
        <div>
          <p className="text-sm font-semibold text-white">#{detail.post.id}</p>
          <p className="text-xs text-app-muted">
            {detail.post.width}×{detail.post.height} · {formatBytes(detail.file_size)}
          </p>
        </div>
        <AppButton variant="ghost" onClick={openDanbooru}>
          <ExternalLink aria-hidden="true" className="size-4" />
          {t("providerName")}
        </AppButton>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">
        {detail.post.has_sample ? (
          <DanbooruImage
            postId={detail.post.id}
            variant="sample"
            alt={t("postImageAlt", { id: detail.post.id })}
            className="aspect-video max-h-[46vh] w-full bg-app-bg"
            blurred={shouldBlurRating(detail.post.rating, blurSensitive)}
            eager
          />
        ) : null}
        <div className="grid gap-4 p-3">
          <dl className="grid grid-cols-[90px_minmax(0,1fr)] gap-x-2 gap-y-1 text-xs">
            <dt className="text-app-muted">{t("createdAt")}</dt>
            <dd>{detail.created_at}</dd>
            <dt className="text-app-muted">{t("score")}</dt>
            <dd>
              {detail.post.score} · {t("favorites", { count: detail.post.favorite_count })}
            </dd>
            <dt className="text-app-muted">{t("source")}</dt>
            <dd className="min-w-0">
              {sourceUrl ? (
                <button
                  type="button"
                  className="max-w-full truncate text-left text-brand-200 hover:underline"
                  title={sourceUrl}
                  onClick={openSource}
                >
                  {safeHostname(sourceUrl)}
                </button>
              ) : (
                t("noSource")
              )}
            </dd>
          </dl>
          {groupedTags.map(({ category, tags }) =>
            tags.length > 0 ? (
              <TagGroup
                key={category}
                category={category}
                tags={tags}
                selectedTags={selectedTags}
                onToggleTag={onToggleTag}
                onSearchTag={onSearchTag}
              />
            ) : null,
          )}
        </div>
      </div>
    </AppPanel>
  );
}

function TagGroup({
  category,
  tags,
  selectedTags,
  onToggleTag,
  onSearchTag,
}: {
  category: DanbooruTagCategoryDto;
  tags: DanbooruTagDto[];
  selectedTags: ReadonlySet<string>;
  onToggleTag: (tag: DanbooruTagDto) => void;
  onSearchTag: (tag: string) => void;
}) {
  const { t } = useTranslation("explore");
  const toggleTag = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const index = Number(event.currentTarget.dataset.index);
      const tag = tags[index];
      if (tag) onToggleTag(tag);
    },
    [onToggleTag, tags],
  );
  const searchTag = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const name = event.currentTarget.dataset.tag;
      if (name) onSearchTag(name);
    },
    [onSearchTag],
  );
  return (
    <section>
      <h3 className="mb-2 text-xs font-semibold tracking-wide text-app-muted uppercase">
        {t(`categories.${category}`)} · {tags.length}
      </h3>
      <div className="flex flex-wrap gap-1.5">
        {tags.map((tag, index) => (
          <span
            key={`${tag.category}:${tag.canonical_name}`}
            className={[
              "inline-flex min-w-0 items-stretch border text-xs",
              selectedTags.has(tag.canonical_name)
                ? "border-brand-400 bg-brand-500/20 text-brand-100"
                : "border-app-border bg-black/20 text-app-text",
            ].join(" ")}
          >
            <button
              type="button"
              data-index={index}
              className="min-w-0 px-2 py-1 text-left"
              title={tag.translation ?? tag.canonical_name}
              onClick={toggleTag}
            >
              <span className="block truncate">{tag.canonical_name}</span>
              {tag.translation ? (
                <span className="block truncate text-[10px] text-app-muted">{tag.translation}</span>
              ) : null}
            </button>
            <button
              type="button"
              data-tag={tag.canonical_name}
              className="grid w-7 place-items-center border-l border-app-border text-app-muted hover:text-brand-100"
              aria-label={t("searchTag", { tag: tag.canonical_name })}
              onClick={searchTag}
            >
              <Search aria-hidden="true" className="size-3" />
            </button>
          </span>
        ))}
      </div>
    </section>
  );
}

function safeHostname(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}
