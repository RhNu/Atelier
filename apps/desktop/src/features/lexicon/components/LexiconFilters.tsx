import { Search } from "lucide-react";
import { type ChangeEvent, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel, AppTabs } from "@/components/ui";
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
  onModeChange: (value: LexiconSearchModeDto) => void;
  onQueryChange: (value: string) => void;
  onKindChange: (value: "all" | LexiconEntityKindDto) => void;
  onCategoryChange: (value: "all" | LexiconCategoryDto) => void;
  onGroupChange: (value: string) => void;
  onRatingChange: (value: LexiconRatingFilter) => void;
};

export function LexiconFilters({
  bootstrap,
  mode,
  query,
  kind,
  category,
  groupId,
  rating,
  onModeChange,
  onQueryChange,
  onKindChange,
  onCategoryChange,
  onGroupChange,
  onRatingChange,
}: Props) {
  const { t } = useTranslation("lexicon");
  const tabs = useMemo(
    () => [
      { value: "lexical" as const, label: t("fastSearch") },
      {
        value: "semantic" as const,
        label: bootstrap?.status.semantic_available
          ? t("semanticExplore")
          : t("semanticUnavailable"),
        disabled: !bootstrap?.status.semantic_available,
      },
    ],
    [bootstrap?.status.semantic_available, t],
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
  const handleQueryChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onQueryChange(event.target.value),
    [onQueryChange],
  );

  return (
    <AppPanel variant="section" className="border-b border-app-border p-3">
      <div className="grid gap-3 lg:grid-cols-[260px_minmax(260px,1fr)_repeat(4,minmax(120px,180px))]">
        <AppTabs value={mode} label={t("searchMode")} tabs={tabs} onChange={onModeChange} />
        <label className="relative block">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-app-muted"
          />
          <input
            aria-label={t("searchTags")}
            value={query}
            onChange={handleQueryChange}
            className="h-9 w-full border border-app-border bg-black/20 pr-3 pl-9 text-sm text-app-text outline-none focus:border-brand-400"
            placeholder={mode === "semantic" ? t("semanticPlaceholder") : t("searchPlaceholder")}
          />
        </label>
        <FilterSelect
          label={t("entityType")}
          value={kind}
          onChange={onKindChange}
          options={kindOptions}
        />
        <FilterSelect
          label={t("category")}
          value={category}
          onChange={onCategoryChange}
          options={categoryOptions}
        />
        <FilterSelect
          label={t("group")}
          value={groupId}
          onChange={onGroupChange}
          options={groupOptions}
        />
        <FilterSelect
          label={t("rating")}
          value={rating}
          onChange={onRatingChange}
          options={ratingOptions}
        />
      </div>
    </AppPanel>
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
  onChange,
}: {
  label: string;
  value: TValue;
  options: ReadonlyArray<readonly [TValue, string]>;
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
        onChange={handleChange}
        className="h-7 min-w-0 border border-app-border bg-app-surface px-2 text-xs text-app-text outline-none focus:border-brand-400"
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
