import { KeyRound, RotateCcw, Save } from "lucide-react";

import { AppButton, AppPanel, AppToolbar, EmptyState } from "../../components/ui";
import { useWorkspaceSettingsQuery } from "./data/useWorkspaceSettingsQuery";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function SettingsPage() {
  const settingsQuery = useWorkspaceSettingsQuery();
  const generation = settingsQuery.data?.generation;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold uppercase text-brand-200">Settings</p>
          <h1 className="text-lg font-semibold text-white">Workspace Settings</h1>
        </div>
        <div className="flex items-center gap-2">
          <AppButton variant="ghost">
            <RotateCcw aria-hidden="true" className="size-4" />
            Reset
          </AppButton>
          <AppButton>
            <Save aria-hidden="true" className="size-4" />
            Save
          </AppButton>
        </div>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-3 p-3">
        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">Generation Defaults</h2>
          </header>
          <div className="p-3">
            {settingsQuery.isPending ? (
              <p className="text-sm text-app-muted">Loading settings</p>
            ) : settingsQuery.isError ? (
              <EmptyState
                title="Settings unavailable"
                description={formatError(settingsQuery.error)}
              />
            ) : generation ? (
              <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
                <SettingMetric label="Model" value={generation.model} />
                <SettingMetric
                  label="Size"
                  value={`${generation.size.width} x ${generation.size.height}`}
                />
                <SettingMetric label="Sampler" value={generation.sampler} />
                <SettingMetric label="Steps" value={String(generation.steps)} />
                <SettingMetric label="Scale" value={String(generation.scale)} />
                <SettingMetric label="Samples" value={String(generation.n_samples)} />
              </div>
            ) : (
              <EmptyState title="No workspace settings" />
            )}
          </div>
        </AppPanel>

        <AppPanel className="min-h-0 overflow-hidden">
          <header className="border-b border-app-border px-4 py-3">
            <h2 className="text-sm font-semibold text-white">NovelAI Account</h2>
          </header>
          <div className="grid gap-3 p-3">
            <div className="border border-app-border bg-app-surface p-3">
              <KeyRound aria-hidden="true" className="mb-3 size-5 text-app-muted" />
              <p className="text-sm font-semibold text-app-text">API key registry</p>
              <p className="mt-2 text-sm text-app-muted">No key selected</p>
            </div>
          </div>
        </AppPanel>
      </div>
    </div>
  );
}

function SettingMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="border border-app-border bg-app-surface p-3">
      <p className="text-xs uppercase text-app-muted">{label}</p>
      <p className="mt-2 truncate text-sm font-semibold text-app-text">{value}</p>
    </div>
  );
}
