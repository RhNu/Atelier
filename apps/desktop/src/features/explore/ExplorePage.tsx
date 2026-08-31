import { UserRound } from "lucide-react";
import { useCallback, useState, type ChangeEvent, type ComponentType } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppHelpMarker, EmptyState } from "@/components/ui";
import type { ExploreSourceIdDto } from "@/types";

import { DanbooruBrowser } from "./DanbooruBrowser";
import { useDanbooruAccountQuery } from "./data/useDanbooruQueries";
import { useExploreSources } from "./data/useExploreQueries";
import { NovelAiExploreBrowser } from "./NovelAiExploreBrowser";
import { ExploreActiveContext } from "./state/explore-active";

const SOURCE_VIEWS: Record<ExploreSourceIdDto, ComponentType<{ active: boolean }>> = {
  danbooru_database: DanbooruBrowser,
  novelai_explore_gallery: NovelAiExploreBrowser,
};

function initialSource(): ExploreSourceIdDto {
  return window.localStorage.getItem("atelier.explore.source.v1") === "novelai_explore_gallery"
    ? "novelai_explore_gallery"
    : "danbooru_database";
}

export function ExplorePage() {
  const { t } = useTranslation("explore");
  const sources = useExploreSources();
  const [selected, setSelected] = useState(initialSource);
  const [visited, setVisited] = useState<ExploreSourceIdDto[]>(() => [initialSource()]);
  const selectedSource = sources.data?.find((source) => source.id === selected);
  const account = useDanbooruAccountQuery(
    selected === "danbooru_database" && selectedSource?.available === true,
  );
  const choose = useCallback((event: ChangeEvent<HTMLSelectElement>) => {
    const id = event.target.value;
    if (id !== "danbooru_database" && id !== "novelai_explore_gallery") return;
    setSelected(id);
    setVisited((previous) => (previous.includes(id) ? previous : [...previous, id]));
    window.localStorage.setItem("atelier.explore.source.v1", id);
  }, []);
  const retry = useCallback(() => {
    void sources.refetch();
  }, [sources]);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-app-border bg-app-panel px-3">
        <div className="flex items-center gap-1.5">
          <label className="sr-only" htmlFor="explore-source">
            {t("source")}
          </label>
          <select
            id="explore-source"
            value={selected}
            onChange={choose}
            disabled={!sources.data}
            className="h-8 border border-app-border bg-app-bg px-2 text-xs"
          >
            {sources.data?.map((source) => (
              <option key={source.id} value={source.id}>
                {source.name}
              </option>
            ))}
          </select>
          {selected === "danbooru_database" ? (
            <AppHelpMarker label={t("syntaxHelpLabel")} content={t("syntaxHint")} hoverOnly />
          ) : selected === "novelai_explore_gallery" ? (
            <AppHelpMarker
              label={t("novelai.searchHelpLabel")}
              content={t("novelai.searchHint")}
              hoverOnly
            />
          ) : null}
        </div>
        {selected === "danbooru_database" && selectedSource?.available === true ? (
          <div className="flex items-center gap-2 text-xs text-app-muted">
            <UserRound aria-hidden="true" className="size-4" />
            {account.data?.configured
              ? (account.data.username ?? t("configuredAccount"))
              : t("anonymousMode")}
          </div>
        ) : null}
      </header>
      {sources.isError ? (
        <>
          <EmptyState title={t("sourcesFailed")} />
          <AppButton onClick={retry}>{t("retry")}</AppButton>
        </>
      ) : null}
      {sources.isPending ? (
        <p className="p-3 text-sm text-app-muted">{t("loadingSources")}</p>
      ) : null}
      {visited.map((id) => {
        const source = sources.data?.find((value) => value.id === id);
        if (!source) return null;
        const active = id === selected;
        const View = SOURCE_VIEWS[id];
        return (
          <section key={id} hidden={!active} className={active ? "min-h-0 flex-1" : "hidden"}>
            {source.available ? (
              <ExploreActiveContext.Provider value={active}>
                <View active={active} />
              </ExploreActiveContext.Provider>
            ) : (
              <EmptyState title={t("sourceUnavailable")} />
            )}
          </section>
        );
      })}
    </div>
  );
}
