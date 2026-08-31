import { useCallback, useState, type ChangeEvent, type FormEvent, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { LexiconSearchItemDto, NovelAiExplorePeriodDto, NovelAiExploreSortDto } from "@/types";

type Props = {
  tags: string;
  sort: NovelAiExploreSortDto;
  period: NovelAiExplorePeriodDto;
  creator: string;
  searching: boolean;
  suggestions: LexiconSearchItemDto[];
  onTags: (value: string) => void;
  onSort: (value: NovelAiExploreSortDto) => void;
  onPeriod: (value: NovelAiExplorePeriodDto) => void;
  onCreator: (value: string) => void;
  onSuggestion: (name: string) => void;
  onSubmit: () => void;
};

export function NovelAiExploreToolbar(props: Props) {
  const { onSubmit, onTags, onCreator, onSort, onPeriod, onSuggestion } = props;
  const { t } = useTranslation("explore");
  const [focused, setFocused] = useState(false);
  const random = props.sort === "random";
  const submit = useCallback(
    (event: FormEvent) => {
      event.preventDefault();
      setFocused(false);
      onSubmit();
    },
    [onSubmit],
  );
  const tags = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onTags(event.target.value),
    [onTags],
  );
  const creator = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onCreator(event.target.value),
    [onCreator],
  );
  const sort = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const value = event.target.value;
      if (value === "new" || value === "top" || value === "hot" || value === "random")
        onSort(value);
    },
    [onSort],
  );
  const period = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const value = event.target.value;
      if (value === "day" || value === "week" || value === "month") onPeriod(value);
    },
    [onPeriod],
  );
  const focus = useCallback(() => setFocused(true), []);
  const blur = useCallback(() => setFocused(false), []);
  const suggestion = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const tag = event.currentTarget.dataset.tag;
      if (tag) onSuggestion(tag);
    },
    [onSuggestion],
  );
  const keepFocus = useCallback((event: MouseEvent) => event.preventDefault(), []);
  return (
    <form
      onSubmit={submit}
      className="grid shrink-0 gap-2 border-b border-app-border bg-app-panel p-3"
    >
      <div className="flex items-start gap-2">
        <div className="relative min-w-0 flex-1">
          <input
            aria-label={t("novelai.tags")}
            placeholder={t("novelai.tagsPlaceholder")}
            value={props.tags}
            onChange={tags}
            onFocus={focus}
            onBlur={blur}
            disabled={random}
            autoComplete="off"
            className="h-9 w-full border border-app-border bg-app-bg px-2 text-sm disabled:opacity-40"
          />
          {focused && !random && props.suggestions.length > 0 ? (
            <div className="absolute top-full right-0 left-0 z-20 border border-app-border bg-app-panel">
              {props.suggestions.map((item) => (
                <button
                  key={item.entity_id}
                  type="button"
                  data-tag={item.canonical_name}
                  onMouseDown={keepFocus}
                  onClick={suggestion}
                  className="flex w-full justify-between gap-2 px-2 py-1.5 text-left text-xs hover:bg-app-surface"
                >
                  <span>{item.canonical_name}</span>
                  <span className="text-app-muted">{item.primary_translation}</span>
                </button>
              ))}
            </div>
          ) : null}
        </div>
        <AppButton type="submit" disabled={props.searching}>
          {props.searching ? t("searching") : random ? t("novelai.shuffle") : t("search")}
        </AppButton>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <select
          aria-label={t("novelai.sort")}
          value={props.sort}
          onChange={sort}
          className="h-8 border border-app-border bg-app-bg px-2 text-xs"
        >
          {(["new", "top", "hot", "random"] as const).map((value) => (
            <option key={value} value={value}>
              {t(`novelai.sorts.${value}`)}
            </option>
          ))}
        </select>
        <select
          aria-label={t("novelai.period")}
          value={props.period}
          onChange={period}
          disabled={props.sort === "new"}
          className="h-8 border border-app-border bg-app-bg px-2 text-xs disabled:opacity-40"
        >
          {(["day", "week", "month"] as const).map((value) => (
            <option key={value} value={value}>
              {t(`novelai.periods.${value}`)}
            </option>
          ))}
        </select>
        <input
          aria-label={t("novelai.creator")}
          placeholder={t("novelai.creatorPlaceholder")}
          value={props.creator}
          onChange={creator}
          disabled={random}
          className="h-8 min-w-0 flex-1 border border-app-border bg-app-bg px-2 text-xs disabled:opacity-40"
        />
      </div>
      {random ? <p className="text-xs text-app-muted">{t("novelai.randomHint")}</p> : null}
    </form>
  );
}
