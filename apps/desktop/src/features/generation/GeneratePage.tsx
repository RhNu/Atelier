import { Loader2, Pause, Play, Square, WandSparkles } from "lucide-react";

import {
  AppButton,
  AppIconButton,
  AppPanel,
  AppToolbar,
  EmptyState,
  ResourceImage,
} from "../../components/ui";
import {
  useGenerationStatusQuery,
  useLatestRunHistoryQuery,
} from "./data/useGenerationStatusQuery";
import { useGenerationDraftStore } from "../../stores/workspace-ui-store";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function GeneratePage() {
  const statusQuery = useGenerationStatusQuery();
  const historyQuery = useLatestRunHistoryQuery();
  const prompt = useGenerationDraftStore((state) => state.prompt);
  const negativePrompt = useGenerationDraftStore((state) => state.negativePrompt);
  const setPrompt = useGenerationDraftStore((state) => state.setPrompt);
  const setNegativePrompt = useGenerationDraftStore((state) => state.setNegativePrompt);
  const status = statusQuery.data;
  const historyItems = historyQuery.data?.items ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold uppercase text-brand-200">Generate</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Image Workspace</h1>
        </div>
        <div className="flex items-center gap-2">
          <AppIconButton icon={Pause} label="Pause queue" />
          <AppIconButton icon={Play} label="Resume queue" />
          <AppIconButton icon={Square} label="Stop queue" />
          <AppButton>
            <WandSparkles aria-hidden="true" className="size-4" />
            Queue generation
          </AppButton>
        </div>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-3 p-3">
        <AppPanel className="grid min-h-0 grid-rows-[minmax(0,1fr)_auto] overflow-hidden">
          <div className="min-h-0 bg-black/30 p-4">
            {statusQuery.isPending ? (
              <div className="flex h-full items-center justify-center text-app-muted">
                <Loader2 aria-hidden="true" className="mr-2 size-4 animate-spin" />
                Checking queue
              </div>
            ) : statusQuery.isError ? (
              <EmptyState
                title="Generation status unavailable"
                description={formatError(statusQuery.error)}
              />
            ) : (
              <ResourceImage
                src={null}
                fallbackLabel="No active preview"
                className="h-full min-h-[320px] w-full"
              />
            )}
          </div>
          <div className="grid grid-cols-3 border-t border-app-border text-sm">
            <div className="border-r border-app-border p-3">
              <p className="text-xs uppercase text-app-muted">Batch</p>
              <p className="mt-1 font-semibold text-app-text">{status?.batch_status ?? "idle"}</p>
            </div>
            <div className="border-r border-app-border p-3">
              <p className="text-xs uppercase text-app-muted">Job</p>
              <p className="mt-1 font-semibold text-app-text">{status?.job_status ?? "idle"}</p>
            </div>
            <div className="p-3">
              <p className="text-xs uppercase text-app-muted">History</p>
              <p className="mt-1 font-semibold text-app-text">{historyItems.length} recent</p>
            </div>
          </div>
        </AppPanel>

        <aside className="grid min-h-0 grid-rows-[minmax(0,1fr)_240px] gap-3">
          <AppPanel className="flex min-h-0 flex-col overflow-hidden">
            <header className="border-b border-app-border px-4 py-3">
              <h2 className="text-sm font-semibold text-white">Prompt Stack</h2>
            </header>
            <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto p-3">
              <label className="grid gap-2 text-xs font-semibold uppercase text-app-muted">
                Positive prompt
                <textarea
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                  className="min-h-40 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal normal-case text-app-text outline-none focus:border-brand-400"
                />
              </label>
              <label className="grid gap-2 text-xs font-semibold uppercase text-app-muted">
                Undesired content
                <textarea
                  value={negativePrompt}
                  onChange={(event) => setNegativePrompt(event.target.value)}
                  className="min-h-24 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal normal-case text-app-text outline-none focus:border-brand-400"
                />
              </label>
            </div>
          </AppPanel>

          <AppPanel className="min-h-0 overflow-hidden">
            <header className="border-b border-app-border px-4 py-3">
              <h2 className="text-sm font-semibold text-white">Recent Runs</h2>
            </header>
            <div className="h-[188px] overflow-auto p-2">
              {historyQuery.isPending ? (
                <p className="p-3 text-sm text-app-muted">Loading history</p>
              ) : historyQuery.isError ? (
                <p className="p-3 text-sm text-rose-100">{formatError(historyQuery.error)}</p>
              ) : historyItems.length === 0 ? (
                <EmptyState title="No runs" />
              ) : (
                <div className="grid gap-2">
                  {historyItems.map((item) => (
                    <article
                      key={item.run_id}
                      className="border border-app-border bg-app-surface/65 p-3"
                    >
                      <p className="text-sm font-semibold text-app-text">
                        {item.title ?? item.run_id}
                      </p>
                      <p className="mt-1 text-xs text-app-muted">
                        {item.kind} / {item.status}
                      </p>
                    </article>
                  ))}
                </div>
              )}
            </div>
          </AppPanel>
        </aside>
      </div>
    </div>
  );
}
