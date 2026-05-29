import { AppPanel } from "../../../components/ui";

const disabledTools = [
  "Image to image",
  "Vibe transfer",
  "Director tools",
  "Character references",
] as const;

export function AdvancedPlaceholders() {
  return (
    <AppPanel className="overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Advanced Inputs</h2>
      </header>
      <div className="grid gap-2 p-3">
        {disabledTools.map((tool) => (
          <button
            key={tool}
            type="button"
            aria-label={tool}
            disabled
            className="flex h-10 items-center justify-between border border-app-border bg-app-surface/55 px-3 text-left text-sm text-app-muted disabled:cursor-not-allowed disabled:opacity-60"
          >
            <span>{tool}</span>
            <span className="text-xs uppercase">Planned</span>
          </button>
        ))}
      </div>
    </AppPanel>
  );
}
