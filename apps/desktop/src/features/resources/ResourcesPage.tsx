import { Boxes, Import } from "lucide-react";

import { AppButton, AppPanel, AppToolbar, EmptyState, ResourceImage } from "../../components/ui";
import { usePromptChunksQuery } from "./data/usePromptChunksQuery";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function ResourcesPage() {
  const chunksQuery = usePromptChunksQuery();
  const chunks = chunksQuery.data?.items ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Resources</p>
          <h1 className="text-lg font-semibold text-white">Prompt and Image Resources</h1>
        </div>
        <AppButton variant="secondary">
          <Import aria-hidden="true" className="size-4" />
          Import
        </AppButton>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)] gap-3 p-3">
        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Libraries</h2>
          </header>
          <div className="grid gap-2 p-3 text-sm">
            {["Prompt chunks", "Image references", "Vibe documents", "Masks"].map((item) => (
              <button
                key={item}
                type="button"
                className="flex items-center gap-3 border border-app-border bg-app-surface px-3 py-2 text-left text-app-text"
              >
                <Boxes aria-hidden="true" className="size-4 text-app-muted" />
                {item}
              </button>
            ))}
          </div>
        </AppPanel>

        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Prompt Chunks</h2>
          </header>
          <div className="h-full min-h-0 overflow-auto p-3">
            {chunksQuery.isPending ? (
              <p className="text-sm text-app-muted">Loading prompt chunks</p>
            ) : chunksQuery.isError ? (
              <EmptyState
                title="Prompt chunks unavailable"
                description={formatError(chunksQuery.error)}
              />
            ) : chunks.length === 0 ? (
              <EmptyState title="No prompt chunks" />
            ) : (
              <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
                {chunks.map((chunk) => (
                  <article key={chunk.chunk_id} className="border border-app-border bg-app-surface">
                    <ResourceImage
                      src={null}
                      fallbackLabel={chunk.category ?? "Prompt"}
                      className="h-32 w-full"
                    />
                    <div className="p-3">
                      <p className="text-sm font-semibold text-app-text">{chunk.key}</p>
                      <p className="mt-2 line-clamp-3 text-sm text-app-muted">{chunk.content}</p>
                    </div>
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
