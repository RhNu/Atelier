import { Search } from "lucide-react";
import { useCallback, useState, type ChangeEvent, type FormEvent, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { LexiconSearchItemDto } from "@/types";

type Props = {
  query: string;
  showAdult: boolean;
  suggestions: LexiconSearchItemDto[];
  validationError: string | null;
  searching: boolean;
  onQueryChange: (query: string) => void;
  onAdultChange: (showAdult: boolean) => void;
  onSuggestion: (canonicalName: string) => void;
  onSubmit: () => void;
};

export function InspirationSearchToolbar({
  query,
  showAdult,
  suggestions,
  validationError,
  searching,
  onQueryChange,
  onAdultChange,
  onSuggestion,
  onSubmit,
}: Props) {
  const { t } = useTranslation("inspiration");
  const [focused, setFocused] = useState(false);
  const submit = useCallback(
    (event: FormEvent) => {
      event.preventDefault();
      onSubmit();
    },
    [onSubmit],
  );
  const changeQuery = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onQueryChange(event.target.value),
    [onQueryChange],
  );
  const focusSearch = useCallback(() => setFocused(true), []);
  const blurSearch = useCallback(() => {
    window.setTimeout(() => setFocused(false), 120);
  }, []);
  const keepFocus = useCallback((event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
  }, []);
  const chooseSuggestion = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const name = event.currentTarget.dataset.tag;
      if (name) onSuggestion(name);
    },
    [onSuggestion],
  );
  const changeAdult = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onAdultChange(event.target.checked),
    [onAdultChange],
  );

  return (
    <form onSubmit={submit} className="border-b border-app-border bg-app-panel p-3">
      <div className="flex items-start gap-2">
        <div className="relative min-w-0 flex-1">
          <label className="sr-only" htmlFor="danbooru-search">
            {t("searchLabel")}
          </label>
          <input
            id="danbooru-search"
            aria-label={t("searchLabel")}
            value={query}
            onChange={changeQuery}
            onFocus={focusSearch}
            onBlur={blurSearch}
            placeholder={t("searchPlaceholder")}
            autoComplete="off"
            className="h-9 w-full border border-app-border bg-app-bg px-3 text-sm text-app-text outline-none focus:border-brand-400"
          />
          {focused && suggestions.length > 0 ? (
            <div className="absolute top-full right-0 left-0 z-30 max-h-64 overflow-auto border border-app-border bg-app-panel shadow-app-panel">
              {suggestions.map((item) => (
                <button
                  key={item.entity_id}
                  type="button"
                  data-tag={item.canonical_name}
                  className="flex w-full items-center justify-between gap-3 border-b border-app-border px-3 py-2 text-left text-xs last:border-b-0 hover:bg-app-surface"
                  onMouseDown={keepFocus}
                  onClick={chooseSuggestion}
                >
                  <span className="font-semibold text-brand-100">{item.canonical_name}</span>
                  <span className="truncate text-app-muted">{item.primary_translation}</span>
                </button>
              ))}
            </div>
          ) : null}
          <p className="min-h-5 pt-1 text-xs text-rose-200">{validationError ?? ""}</p>
        </div>
        <AppButton type="submit" disabled={searching || validationError !== null}>
          <Search aria-hidden="true" className="size-4" />
          {searching ? t("searching") : t("search")}
        </AppButton>
      </div>
      <div className="flex items-center justify-between gap-4 text-xs text-app-muted">
        <p>{t("syntaxHint")}</p>
        <label className="flex shrink-0 items-center gap-2">
          <input
            type="checkbox"
            aria-label={t("showAdult")}
            checked={showAdult}
            onChange={changeAdult}
          />
          {t("showAdult")}
        </label>
      </div>
    </form>
  );
}
