import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, EmptyState } from "@/components/ui";
import type { NovelAiExplorePostSummaryDto } from "@/types";

import { ExploreImage } from "./ExploreImage";

type Props = {
  items: NovelAiExplorePostSummaryDto[];
  selected: string | null;
  blurSensitive: boolean;
  hasNextPage: boolean;
  loadingMore: boolean;
  error: unknown;
  onSelect: (id: string) => void;
  onLoadMore: () => void;
};

export function NovelAiExploreResults(props: Props) {
  const { t } = useTranslation("explore");
  return (
    <div className="grid gap-3">
      {props.items.length === 0 ? <EmptyState title={t("noResults")} /> : null}
      {props.error ? (
        <output className="text-xs text-app-muted">{t("loadMoreFailed")}</output>
      ) : null}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-3">
        {props.items.map((item) => (
          <NovelAiCard
            key={item.id}
            item={item}
            selected={item.id === props.selected}
            blurred={props.blurSensitive}
            onSelect={props.onSelect}
          />
        ))}
      </div>
      {props.hasNextPage ? (
        <AppButton
          variant="secondary"
          className="mx-auto"
          disabled={props.loadingMore}
          onClick={props.onLoadMore}
        >
          {props.loadingMore ? t("loadingMore") : t("loadMore")}
        </AppButton>
      ) : (
        <p className="text-center text-xs text-app-muted">{t("endOfResults")}</p>
      )}
    </div>
  );
}

function NovelAiCard({
  item,
  selected,
  blurred,
  onSelect,
}: {
  item: NovelAiExplorePostSummaryDto;
  selected: boolean;
  blurred: boolean;
  onSelect: (id: string) => void;
}) {
  const { t } = useTranslation("explore");
  const select = useCallback(() => onSelect(item.id), [item.id, onSelect]);
  return (
    <button
      type="button"
      onClick={select}
      aria-label={t("novelai.selectPost", { title: item.title || item.id })}
      className={[
        "min-w-0 overflow-hidden border bg-app-surface text-left",
        selected ? "border-brand-400" : "border-app-border hover:border-brand-400/60",
      ].join(" ")}
    >
      <ExploreImage
        sourceId="novelai_explore_gallery"
        itemId={item.id}
        variant="thumbnail"
        alt={item.title}
        className="aspect-square w-full overflow-hidden bg-app-bg"
        blurred={blurred}
      />
      <div className="grid gap-1 p-2">
        <p className="truncate text-xs font-semibold">{item.title || t("novelai.untitled")}</p>
        <p className="truncate text-xs text-app-muted">
          {item.creator_name ?? t("novelai.unknownCreator")}
        </p>
        <p className="text-[10px] text-app-muted">
          {item.width}×{item.height}
          {item.like_count !== null ? ` · ${t("novelai.likes", { count: item.like_count })}` : ""}
        </p>
      </div>
    </button>
  );
}
