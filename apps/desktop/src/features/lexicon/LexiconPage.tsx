import { useCallback, useDeferredValue, useState, type ChangeEvent } from "react";

import { AppTabs, AppToolbar } from "../../components/ui";
import type { PromptLexiconListQueryDto } from "../../types";
import { LexiconResults } from "./components/LexiconResults";
import { LexiconSidebar, type LexiconCategorySelection } from "./components/LexiconSidebar";
import {
  LEXICON_BROWSE_LIMIT,
  usePromptLexiconBrowseQuery,
  usePromptLexiconCatalogQuery,
  usePromptLexiconSearchQuery,
} from "./data/usePromptLexiconQueries";

type LexiconView = "catalog" | "search";

const lexiconViewTabs = [
  { value: "catalog", label: "Catalog" },
  { value: "search", label: "Search" },
] as const;
const EMPTY_SELECTION: LexiconCategorySelection = { category: null, subcategory: null };

export function LexiconPage() {
  const [view, setView] = useState<LexiconView>("catalog");
  const [search, setSearch] = useState("");
  const [selection, setSelection] = useState(EMPTY_SELECTION);
  const [offset, setOffset] = useState(0);
  const deferredSearch = useDeferredValue(search.trim());
  const browseRequest: PromptLexiconListQueryDto = {
    query: "",
    category: selection.category,
    subcategory: selection.subcategory,
    offset,
    limit: LEXICON_BROWSE_LIMIT,
  };
  const catalogQuery = usePromptLexiconCatalogQuery();
  const browseQuery = usePromptLexiconBrowseQuery(browseRequest, view === "catalog");
  const searchQuery = usePromptLexiconSearchQuery(deferredSearch, view === "search");

  const handleViewChange = useCallback((value: string) => {
    if (value === "catalog" || value === "search") setView(value);
  }, []);
  const handleSearchChange = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearch(value);
    if (value.trim()) setView("search");
  }, []);
  const handleSelectionChange = useCallback((next: LexiconCategorySelection) => {
    setSelection(next);
    setOffset(0);
    setView("catalog");
  }, []);
  const handlePrevious = useCallback(() => {
    setOffset((current) => Math.max(0, current - LEXICON_BROWSE_LIMIT));
  }, []);
  const handleNext = useCallback(() => {
    setOffset((current) => current + LEXICON_BROWSE_LIMIT);
  }, []);

  const showingSearch = view === "search";
  const activePage = showingSearch ? searchQuery.data : browseQuery.data;
  const activePending = showingSearch
    ? deferredSearch.length > 0 && searchQuery.isPending
    : browseQuery.isPending;
  const activeError = showingSearch ? searchQuery.error : browseQuery.error;
  const title = showingSearch
    ? deferredSearch
      ? `Search: ${deferredSearch}`
      : "Search results"
    : (selection.subcategory ?? selection.category ?? "All tags");

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Lexicon</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Prompt Lexicon</h1>
        </div>
        <AppTabs
          value={view}
          label="Lexicon views"
          tabs={lexiconViewTabs}
          onChange={handleViewChange}
        />
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[320px_minmax(0,1fr)] divide-x divide-app-border">
        <LexiconSidebar
          catalog={catalogQuery.data}
          catalogPending={catalogQuery.isPending}
          catalogError={catalogQuery.isError ? formatError(catalogQuery.error) : null}
          search={search}
          selection={selection}
          onSearchChange={handleSearchChange}
          onSelect={handleSelectionChange}
        />
        <LexiconResults
          title={title}
          page={activePage}
          pending={activePending}
          error={activeError ? formatError(activeError) : null}
          emptyTitle={showingSearch ? "Enter a search or try another term" : "No matching tags"}
          pagination={!showingSearch}
          onPrevious={handlePrevious}
          onNext={handleNext}
        />
      </div>
    </div>
  );
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}
