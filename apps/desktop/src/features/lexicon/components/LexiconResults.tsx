import { ChevronLeft, ChevronRight } from "lucide-react";

import { AppButton, AppPanel, EmptyState } from "@/components/ui";
import type { PromptLexiconPageDto } from "@/types";

type LexiconResultsProps = {
  title: string;
  page: PromptLexiconPageDto | undefined;
  pending: boolean;
  error: string | null;
  emptyTitle: string;
  pagination: boolean;
  onPrevious: () => void;
  onNext: () => void;
};

export function LexiconResults({
  title,
  page,
  pending,
  error,
  emptyTitle,
  pagination,
  onPrevious,
  onNext,
}: LexiconResultsProps) {
  const shownFrom = page && page.items.length > 0 ? page.offset + 1 : 0;
  const shownTo = page ? page.offset + page.items.length : 0;
  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex min-h-12 items-center justify-between gap-3 border-b border-app-border px-4 py-2">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold text-white">{title}</h2>
          {page ? (
            <p className="text-xs text-app-muted">
              {shownFrom}-{shownTo} of {page.total}
            </p>
          ) : null}
        </div>
        {pagination && page ? (
          <div className="flex gap-1">
            <AppButton
              variant="ghost"
              className="size-8 p-0"
              aria-label="Previous lexicon page"
              disabled={page.offset === 0 || pending}
              onClick={onPrevious}
            >
              <ChevronLeft aria-hidden="true" className="size-4" />
            </AppButton>
            <AppButton
              variant="ghost"
              className="size-8 p-0"
              aria-label="Next lexicon page"
              disabled={shownTo >= page.total || pending}
              onClick={onNext}
            >
              <ChevronRight aria-hidden="true" className="size-4" />
            </AppButton>
          </div>
        ) : null}
      </header>

      <div className="min-h-0 flex-1 overflow-auto">
        {pending && !page ? (
          <p className="p-4 text-sm text-app-muted">Loading lexicon entries</p>
        ) : error ? (
          <EmptyState title="Lexicon query failed" description={error} />
        ) : !page || page.items.length === 0 ? (
          <EmptyState title={emptyTitle} />
        ) : (
          <div className="divide-y divide-app-border">
            {page.items.map((entry) => (
              <article key={entry.tag} className="grid grid-cols-[minmax(0,1fr)_180px] gap-4 p-3">
                <div className="min-w-0">
                  <div className="flex items-baseline gap-2">
                    <code className="truncate text-sm font-semibold text-brand-100">
                      {entry.tag}
                    </code>
                    {entry.weight === null ? null : (
                      <span className="text-[10px] text-app-muted">weight {entry.weight}</span>
                    )}
                  </div>
                  <p className="mt-1 truncate text-sm text-app-text">
                    {entry.primary_translation || entry.tag}
                  </p>
                  {entry.matched_translation !== entry.primary_translation ? (
                    <p className="mt-1 truncate text-xs text-app-muted">
                      Matched: {entry.matched_translation}
                    </p>
                  ) : null}
                </div>
                <div className="text-right text-xs text-app-muted">
                  <p>{entry.category || "Uncategorized"}</p>
                  <p className="mt-1 truncate">{entry.subcategory || "—"}</p>
                  {entry.match_rank ? (
                    <p className="mt-1 text-[10px] uppercase opacity-70">
                      {entry.match_field} / {entry.match_rank}
                    </p>
                  ) : null}
                </div>
              </article>
            ))}
          </div>
        )}
      </div>
    </AppPanel>
  );
}
