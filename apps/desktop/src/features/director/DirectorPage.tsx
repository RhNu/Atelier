import { Clapperboard, ScanSearch } from "lucide-react";

import { AppButton, AppPanel, AppToolbar, EmptyState } from "../../components/ui";
import { useTemporaryEditorStore } from "../../stores/workspace-ui-store";
import { useDirectorReadinessQuery } from "./data/useDirectorReadinessQuery";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function DirectorPage() {
  const readinessQuery = useDirectorReadinessQuery();
  const directorNote = useTemporaryEditorStore((state) => state.directorNote);
  const setDirectorNote = useTemporaryEditorStore((state) => state.setDirectorNote);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold uppercase text-brand-200">Director</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Director Tools</h1>
        </div>
        <AppButton variant="secondary">
          <ScanSearch aria-hidden="true" className="size-4" />
          Inspect image
        </AppButton>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[320px_minmax(0,1fr)_360px] gap-3 p-3">
        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Input</h2>
          </header>
          <div className="p-3">
            <EmptyState title="No director input" />
          </div>
        </AppPanel>

        <AppPanel className="min-h-0 overflow-hidden bg-black/25">
          <div className="flex h-full items-center justify-center p-6">
            <div className="text-center">
              <Clapperboard aria-hidden="true" className="mx-auto mb-4 size-10 text-app-muted" />
              <h2 className="text-base font-semibold text-white">Director canvas</h2>
              <p className="mt-2 text-sm text-app-muted">No active run</p>
            </div>
          </div>
        </AppPanel>

        <AppPanel className="flex min-h-0 flex-col overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Run State</h2>
          </header>
          <div className="grid gap-3 p-3">
            {readinessQuery.isPending ? (
              <p className="text-sm text-app-muted">Checking active NovelAI account</p>
            ) : readinessQuery.isError ? (
              <p className="text-sm text-rose-100">{formatError(readinessQuery.error)}</p>
            ) : (
              <div className="border border-app-border bg-app-surface p-3 text-sm">
                <p className="text-app-muted">Tier</p>
                <p className="mt-1 font-semibold text-app-text">{readinessQuery.data.tier_name}</p>
                <p className="mt-2 text-app-muted">Anlas</p>
                <p className="mt-1 font-semibold text-app-text">
                  {readinessQuery.data.anlas_balance}
                </p>
              </div>
            )}
            <label className="grid gap-2 text-xs font-semibold uppercase text-app-muted">
              Notes
              <textarea
                value={directorNote}
                onChange={(event) => setDirectorNote(event.target.value)}
                className="min-h-40 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal normal-case text-app-text outline-none focus:border-brand-400"
              />
            </label>
          </div>
        </AppPanel>
      </div>
    </div>
  );
}
