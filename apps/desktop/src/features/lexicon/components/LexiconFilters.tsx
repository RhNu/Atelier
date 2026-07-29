import { Loader2, RotateCcw, Search, X } from "lucide-react";
import { type ChangeEvent, type FormEvent, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton, AppPanel, AppTabs } from "@/components/ui";
import type {
  LexiconBootstrapDto,
  LexiconCategoryDto,
  LexiconEntityKindDto,
  LexiconSearchModeDto,
} from "@/types";

export type LexiconRatingFilter = "all" | "safe" | "sensitive" | "unknown";

type Props = {
  bootstrap: LexiconBootstrapDto | undefined;
  mode: LexiconSearchModeDto;
  query: string;
  kind: "all" | LexiconEntityKindDto;
  category: "all" | LexiconCategoryDto;
  groupId: string;
  rating: LexiconRatingFilter;
  pending: boolean;
  hasActiveSearch: boolean;
  onModeChange: (value: LexiconSearchModeDto) => void;
  onQueryChange: (value: string) => void;
  onQueryClear: () => void;
  onKindChange: (value: "all" | LexiconEntityKindDto) => void;
  onCategoryChange: (value: "all" | LexiconCategoryDto) => void;
  onGroupChange: (value: string) => void;
  onRatingChange: (value: LexiconRatingFilter) => void;
  onReset: () => void;
  onSemanticSubmit: () => void;
};

export function LexiconFilters({
  bootstrap,
  mode,
  query,
  kind,
  category,
  groupId,
  rating,
  pending,
  hasActiveSearch,
  onModeChange,
  onQueryChange,
  onQueryClear,
  onKindChange,
  onCategoryChange,
  onGroupChange,
  onRatingChange,
  onReset,
  onSemanticSubmit,
}: Props) {
  const { t } = useTranslation("lexicon");
  const tabs = useMemo(
    () => [
      { value: "lexical" as const, label: t("fastSearch"), disabled: pending },
      {
        value: "semantic" as const,
        label: bootstrap?.status.semantic_available
          ? t("semanticExplore")
          : t("semanticUnavailable"),
        disabled: pending || !bootstrap?.status.semantic_available,
      },
    ],
    [bootstrap?.status.semantic_available, pending, t],
  );
  const kindOptions = useMemo<ReadonlyArray<readonly [Props["kind"], string]>>(
    () => [
      ["all", t("allEntities")],
      ["tag", t("tags")],
      ["artist", t("artists")],
    ],
    [t],
  );
  const categoryOptions = useMemo<ReadonlyArray<readonly [Props["category"], string]>>(() => {
    const options: Array<readonly [Props["category"], string]> = [["all", t("allCategories")]];
    for (const facet of bootstrap?.categories ?? []) {
      if (isLexiconCategory(facet.value)) {
        options.push([facet.value, `${facet.label} (${facet.count})`]);
      }
    }
    return options;
  }, [bootstrap?.categories, t]);
  const groupOptions = useMemo<ReadonlyArray<readonly [string, string]>>(
    () => [
      ["", t("allGroups")],
      ...(bootstrap?.groups.map((group): readonly [string, string] => [
        group.id,
        `${group.name} (${group.member_count})`,
      ]) ?? []),
    ],
    [bootstrap?.groups, t],
  );
  const ratingOptions = useMemo<ReadonlyArray<readonly [LexiconRatingFilter, string]>>(
    () => [
      ["all", t("ratingAll")],
      ["safe", t("ratingSafe")],
      ["sensitive", t("ratingSensitive")],
      ["unknown", t("ratingUnknown")],
    ],
    [t],
  );
  return (
    <AppPanel variant="section" className="border-b border-app-border p-3">
      <div className="grid grid-cols-2 items-end gap-2 md:grid-cols-4 xl:grid-cols-[minmax(420px,1fr)_repeat(4,minmax(120px,180px))]">
        <LexiconSearchControl
          mode={mode}
          query={query}
          tabs={tabs}
          pending={pending}
          hasActiveSearch={hasActiveSearch}
          onModeChange={onModeChange}
          onQueryChange={onQueryChange}
          onQueryClear={onQueryClear}
          onReset={onReset}
          onSemanticSubmit={onSemanticSubmit}
        />
        <FilterSelect
          label={t("entityType")}
          value={kind}
          disabled={pending}
          onChange={onKindChange}
          options={kindOptions}
        />
        <FilterSelect
          label={t("category")}
          value={category}
          disabled={pending}
          onChange={onCategoryChange}
          options={categoryOptions}
        />
        <FilterSelect
          label={t("group")}
          value={groupId}
          disabled={pending}
          onChange={onGroupChange}
          options={groupOptions}
        />
        <FilterSelect
          label={t("rating")}
          value={rating}
          disabled={pending}
          onChange={onRatingChange}
          options={ratingOptions}
        />
      </div>
    </AppPanel>
  );
}

function LexiconSearchControl({
  mode,
  query,
  tabs,
  pending,
  hasActiveSearch,
  onModeChange,
  onQueryChange,
  onQueryClear,
  onReset,
  onSemanticSubmit,
}: Pick<
  Props,
  | "mode"
  | "query"
  | "pending"
  | "hasActiveSearch"
  | "onModeChange"
  | "onQueryChange"
  | "onQueryClear"
  | "onReset"
  | "onSemanticSubmit"
> & {
  tabs: ReadonlyArray<{
    value: LexiconSearchModeDto;
    label: string;
    disabled?: boolean;
  }>;
}) {
  const { t } = useTranslation("lexicon");
  const handleQueryChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onQueryChange(event.target.value),
    [onQueryChange],
  );
  const handleSubmit = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (mode === "semantic") onSemanticSubmit();
    },
    [mode, onSemanticSubmit],
  );
  return (
    <div className="col-span-2 flex min-w-0 items-center gap-2 md:col-span-4 xl:col-span-1">
      <AppTabs
        value={mode}
        label={t("searchMode")}
        tabs={tabs}
        className="h-9 shrink-0 self-end"
        onChange={onModeChange}
      />
      <form className="flex min-w-0 flex-1 items-center gap-1" onSubmit={handleSubmit}>
        <label className="relative min-w-0 flex-1">
          <SearchStatusIcon pending={pending} />
          <input
            aria-label={t("searchTags")}
            value={query}
            disabled={pending}
            onChange={handleQueryChange}
            className="h-9 w-full border border-app-border bg-black/20 pr-9 pl-9 text-sm text-app-text outline-none focus:border-brand-400 disabled:cursor-wait disabled:opacity-70"
            placeholder={mode === "semantic" ? t("semanticPlaceholder") : t("searchPlaceholder")}
          />
          {query && !pending ? (
            <button
              type="button"
              className="absolute inset-y-0 right-0 flex w-9 items-center justify-center text-app-muted hover:text-app-text"
              aria-label={t("clearSearch")}
              title={t("clearSearch")}
              onClick={onQueryClear}
            >
              <X aria-hidden="true" className="size-[18px]" />
            </button>
          ) : null}
        </label>
        {mode === "semantic" ? (
          <button
            type="submit"
            disabled={pending || query.trim().length === 0}
            className="inline-flex h-9 shrink-0 items-center gap-1.5 border border-brand-500 bg-brand-500 px-3 text-xs font-semibold text-white hover:bg-brand-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <SearchStatusIcon pending={pending} inline />
            {pending ? t("semanticSearching") : t("submitSemantic")}
          </button>
        ) : null}
      </form>
      <AppIconButton
        icon={RotateCcw}
        label={t("resetSearch")}
        size="sm"
        disabled={pending || !hasActiveSearch}
        className="shrink-0 [&>svg]:size-[18px]"
        onClick={onReset}
      />
    </div>
  );
}

function SearchStatusIcon({ pending, inline = false }: { pending: boolean; inline?: boolean }) {
  const Icon = pending ? Loader2 : Search;
  return (
    <Icon
      aria-hidden="true"
      className={[
        "size-[18px]",
        inline ? "" : "pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-app-muted",
        pending ? "animate-spin text-brand-200" : "",
      ].join(" ")}
    />
  );
}

function isLexiconCategory(value: string): value is LexiconCategoryDto {
  return (
    value === "general" || value === "copyright" || value === "character" || value === "artist"
  );
}

function FilterSelect<TValue extends string>({
  label,
  value,
  options,
  disabled = false,
  onChange,
}: {
  label: string;
  value: TValue;
  options: ReadonlyArray<readonly [TValue, string]>;
  disabled?: boolean;
  onChange: (value: TValue) => void;
}) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const selected = options.find(([optionValue]) => optionValue === event.target.value);
      if (selected) onChange(selected[0]);
    },
    [onChange, options],
  );
  return (
    <label className="grid gap-1 text-[10px] tracking-wide text-app-muted uppercase">
      {label}
      <select
        value={value}
        disabled={disabled}
        onChange={handleChange}
        className="h-9 min-w-0 border border-app-border bg-app-surface px-2 text-xs text-app-text outline-none focus:border-brand-400 disabled:cursor-wait disabled:opacity-60"
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </label>
  );
}
