import { Search } from "lucide-react";

import { AppPanel, AppTabs, AppToolbar, EmptyState } from "../../components/ui";
import { useTemporaryEditorStore } from "../../stores/workspace-ui-store";
import { usePromptLexiconCatalogQuery } from "./data/usePromptLexiconCatalogQuery";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function LexiconPage() {
  const catalogQuery = usePromptLexiconCatalogQuery();
  const lexiconSearch = useTemporaryEditorStore((state) => state.lexiconSearch);
  const setLexiconSearch = useTemporaryEditorStore((state) => state.setLexiconSearch);
  const categories = catalogQuery.data?.categories ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold uppercase text-brand-200">Lexicon</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Prompt Lexicon</h1>
        </div>
        <AppTabs
          value="catalog"
          label="Lexicon views"
          tabs={[
            { value: "catalog", label: "Catalog" },
            { value: "search", label: "Search" },
          ]}
          onChange={() => undefined}
        />
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[320px_minmax(0,1fr)] gap-3 p-3">
        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Search</h2>
          </header>
          <div className="p-3">
            <label className="relative block">
              <Search
                aria-hidden="true"
                className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-app-muted"
              />
              <input
                value={lexiconSearch}
                onChange={(event) => setLexiconSearch(event.target.value)}
                className="h-9 w-full border border-app-border bg-black/20 pl-9 pr-3 text-sm text-app-text outline-none focus:border-brand-400"
                placeholder="Search tags"
              />
            </label>
            {catalogQuery.data ? (
              <dl className="mt-4 grid grid-cols-2 gap-2 text-sm">
                <div className="border border-app-border bg-app-surface p-3">
                  <dt className="text-xs uppercase text-app-muted">Tags</dt>
                  <dd className="mt-1 font-semibold text-app-text">
                    {catalogQuery.data.stats.total_tags}
                  </dd>
                </div>
                <div className="border border-app-border bg-app-surface p-3">
                  <dt className="text-xs uppercase text-app-muted">Sources</dt>
                  <dd className="mt-1 font-semibold text-app-text">
                    {catalogQuery.data.stats.source_count}
                  </dd>
                </div>
              </dl>
            ) : null}
          </div>
        </AppPanel>

        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Categories</h2>
          </header>
          <div className="h-full overflow-auto p-3">
            {catalogQuery.isPending ? (
              <p className="text-sm text-app-muted">Loading lexicon</p>
            ) : catalogQuery.isError ? (
              <EmptyState
                title="Lexicon unavailable"
                description={formatError(catalogQuery.error)}
              />
            ) : categories.length === 0 ? (
              <EmptyState title="No lexicon categories" />
            ) : (
              <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
                {categories.map((category) => (
                  <article
                    key={category.name}
                    className="border border-app-border bg-app-surface p-3"
                  >
                    <p className="text-sm font-semibold text-app-text">{category.name}</p>
                    <p className="mt-2 text-xs text-app-muted">
                      {category.tag_count} tags / {category.subcategory_count} subcategories
                    </p>
                  </article>
                ))}
              </div>
            )}
          </div>
        </AppPanel>
      </div>
    </div>
  );
}
