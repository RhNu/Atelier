import { Heart, Star } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, EmptyState } from "@/components/ui";
import type { DanbooruPostSummaryDto } from "@/types";

import { shouldBlurRating, formatError } from "../explore-utils";
import { DanbooruImage } from "./DanbooruImage";

type Props = {
  items: DanbooruPostSummaryDto[];
  selectedId: number | null;
  blurSensitive: boolean;
  pending: boolean;
  error: unknown;
  searched: boolean;
  hasNextPage: boolean;
  loadingMore: boolean;
  onSelect: (postId: number) => void;
  onLoadMore: () => void;
  onRetry: () => void;
};

export function DanbooruResults({
  items,
  selectedId,
  blurSensitive,
  pending,
  error,
  searched,
  hasNextPage,
  loadingMore,
  onSelect,
  onLoadMore,
  onRetry,
}: Props) {
  const { t } = useTranslation("explore");
  if (!searched) return <EmptyState title={t("startTitle")} description={t("startDescription")} />;
  if (pending) return <p className="p-3 text-sm text-app-muted">{t("loadingResults")}</p>;
  if (error && items.length === 0)
    return (
      <>
        <EmptyState title={t("searchFailed")} description={formatError(error)} />
        <AppButton onClick={onRetry}>{t("retry")}</AppButton>
      </>
    );
  if (items.length === 0) return <EmptyState title={t("noResults")} />;

  return (
    <div className="grid gap-3">
      {error ? <output className="text-xs text-app-muted">{t("loadMoreFailed")}</output> : null}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-3">
        {items.map((item) => (
          <DanbooruCard
            key={item.id}
            item={item}
            selected={item.id === selectedId}
            blurSensitive={blurSensitive}
            onSelect={onSelect}
          />
        ))}
      </div>
      {hasNextPage ? (
        <AppButton
          variant="secondary"
          className="mx-auto"
          disabled={loadingMore}
          onClick={onLoadMore}
        >
          {loadingMore ? t("loadingMore") : t("loadMore")}
        </AppButton>
      ) : (
        <p className="py-2 text-center text-xs text-app-muted">{t("endOfResults")}</p>
      )}
    </div>
  );
}

function DanbooruCard({
  item,
  selected,
  blurSensitive,
  onSelect,
}: {
  item: DanbooruPostSummaryDto;
  selected: boolean;
  blurSensitive: boolean;
  onSelect: (postId: number) => void;
}) {
  const { t } = useTranslation("explore");
  const select = useCallback(() => onSelect(item.id), [item.id, onSelect]);
  return (
    <article
      className={[
        "border bg-app-surface transition-colors",
        selected ? "border-brand-400" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <button
        type="button"
        className="block w-full text-left"
        aria-label={t("selectPost", { id: item.id })}
        onClick={select}
      >
        {item.has_preview ? (
          <DanbooruImage
            postId={item.id}
            variant="preview"
            alt={t("postImageAlt", { id: item.id })}
            className="aspect-square w-full bg-app-bg"
            blurred={shouldBlurRating(item.rating, blurSensitive)}
          />
        ) : (
          <div className="grid aspect-square place-items-center bg-app-bg text-xs text-app-muted">
            {t("imageUnavailable")}
          </div>
        )}
        <div className="grid gap-2 p-2">
          <div className="flex items-center justify-between gap-2">
            <span
              className={[
                "border px-1.5 py-0.5 text-[10px] uppercase",
                ratingClassName(item.rating),
              ].join(" ")}
            >
              {t(`ratings.${item.rating}`)}
            </span>
            <span className="text-[10px] text-app-muted">
              {item.width}×{item.height} · {item.file_extension.toUpperCase()}
            </span>
          </div>
          <div className="flex items-center gap-3 text-xs text-app-muted">
            <span className="flex items-center gap-1">
              <Star aria-hidden="true" className="size-3" />
              {item.score}
            </span>
            <span className="flex items-center gap-1">
              <Heart aria-hidden="true" className="size-3" />
              {item.favorite_count}
            </span>
            <span className="ml-auto">{t("tagCount", { count: item.tag_count })}</span>
          </div>
        </div>
      </button>
    </article>
  );
}

function ratingClassName(rating: DanbooruPostSummaryDto["rating"]): string {
  switch (rating) {
    case "general":
      return "border-emerald-500/50 text-emerald-300";
    case "sensitive":
      return "border-amber-500/50 text-amber-300";
    case "questionable":
      return "border-orange-500/50 text-orange-300";
    case "explicit":
      return "border-red-500/50 text-red-300";
  }
}
