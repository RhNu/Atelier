import { Search } from "lucide-react";
import { useCallback, type ChangeEvent } from "react";

import { AppPanel, AppTabs, EmptyState } from "../../../components/ui";
import type { PromptLexiconCatalogDto } from "../../../types";

export type LexiconCategorySelection = {
  category: string | null;
  subcategory: string | null;
};

type LexiconSidebarProps = {
  catalog: PromptLexiconCatalogDto | undefined;
  catalogPending: boolean;
  catalogError: string | null;
  search: string;
  view: "catalog" | "search";
  selection: LexiconCategorySelection;
  onSearchChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onViewChange: (value: string) => void;
  onSelect: (selection: LexiconCategorySelection) => void;
};

export function LexiconSidebar({
  catalog,
  catalogPending,
  catalogError,
  search,
  view,
  selection,
  onSearchChange,
  onViewChange,
  onSelect,
}: LexiconSidebarProps) {
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden">
      <div className="border-b border-app-border p-3">
        <AppTabs
          value={view}
          label="Lexicon views"
          tabs={LEXICON_VIEW_TABS}
          onChange={onViewChange}
        />
        <label className="relative mt-3 block">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-app-muted"
          />
          <input
            aria-label="Search tags"
            value={search}
            onChange={onSearchChange}
            className="h-9 w-full border border-app-border bg-black/20 pr-3 pl-9 text-sm text-app-text outline-none focus:border-brand-400"
            placeholder="Tag, translation, or alias"
          />
        </label>
        {catalog ? <LexiconStats catalog={catalog} /> : null}
      </div>

      <div className="h-full overflow-auto p-2">
        {catalogPending ? (
          <p className="p-2 text-sm text-app-muted">Loading lexicon catalog</p>
        ) : catalogError ? (
          <EmptyState title="Lexicon unavailable" description={catalogError} />
        ) : catalog ? (
          <CategoryNavigator catalog={catalog} selection={selection} onSelect={onSelect} />
        ) : null}
      </div>
    </AppPanel>
  );
}

const LEXICON_VIEW_TABS = [
  { value: "catalog", label: "Catalog" },
  { value: "search", label: "Search" },
] as const;

function LexiconStats({ catalog }: { catalog: PromptLexiconCatalogDto }) {
  return (
    <dl className="mt-3 grid grid-cols-2 gap-2 text-sm">
      <div className="border border-app-border bg-app-surface p-2">
        <dt className="text-xs text-app-muted uppercase">Tags</dt>
        <dd className="mt-1 font-semibold text-app-text">{catalog.stats.total_tags}</dd>
      </div>
      <div className="border border-app-border bg-app-surface p-2">
        <dt className="text-xs text-app-muted uppercase">Translations</dt>
        <dd className="mt-1 font-semibold text-app-text">{catalog.stats.total_translations}</dd>
      </div>
    </dl>
  );
}

function CategoryNavigator({
  catalog,
  selection,
  onSelect,
}: {
  catalog: PromptLexiconCatalogDto;
  selection: LexiconCategorySelection;
  onSelect: (selection: LexiconCategorySelection) => void;
}) {
  return (
    <nav aria-label="Lexicon categories" className="grid gap-1">
      <CategoryButton
        label="All tags"
        count={catalog.stats.total_tags}
        selected={selection.category === null}
        category={null}
        subcategory={null}
        onSelect={onSelect}
      />
      {catalog.categories.map((category) => (
        <div key={category.name}>
          <CategoryButton
            label={category.name}
            count={category.tag_count}
            selected={selection.category === category.name && selection.subcategory === null}
            category={category.name}
            subcategory={null}
            onSelect={onSelect}
          />
          {selection.category === category.name ? (
            <div className="ml-3 grid border-l border-app-border pl-2">
              {category.subcategories.map((subcategory) => (
                <CategoryButton
                  key={subcategory.name}
                  label={subcategory.name}
                  count={subcategory.tag_count}
                  selected={selection.subcategory === subcategory.name}
                  category={category.name}
                  subcategory={subcategory.name}
                  onSelect={onSelect}
                  compact
                />
              ))}
            </div>
          ) : null}
        </div>
      ))}
    </nav>
  );
}

function CategoryButton({
  label,
  count,
  selected,
  compact = false,
  category,
  subcategory,
  onSelect,
}: {
  label: string;
  count: number;
  selected: boolean;
  compact?: boolean;
  category: string | null;
  subcategory: string | null;
  onSelect: (selection: LexiconCategorySelection) => void;
}) {
  const handleClick = useCallback(() => {
    onSelect({ category, subcategory });
  }, [category, onSelect, subcategory]);
  return (
    <button
      type="button"
      aria-pressed={selected}
      className={[
        "flex w-full items-center justify-between gap-2 border px-2 text-left",
        compact ? "h-7 text-xs" : "h-8 text-sm",
        selected
          ? "border-brand-400/50 bg-brand-500/15 text-brand-100"
          : "border-transparent text-app-muted hover:border-app-border hover:bg-app-surface hover:text-app-text",
      ].join(" ")}
      onClick={handleClick}
    >
      <span className="truncate">{label}</span>
      <span className="shrink-0 font-mono text-[10px] opacity-70">{count}</span>
    </button>
  );
}
