import { CircleHelp } from "lucide-react";

export function AppHelpMarker({ content, label = "Help" }: { content: string; label?: string }) {
  return (
    <span className="group relative inline-flex shrink-0">
      <button
        type="button"
        aria-label={label}
        className="grid size-5 place-items-center text-app-muted outline-none hover:text-app-text focus-visible:text-app-text"
      >
        <CircleHelp aria-hidden="true" className="size-3.5" />
      </button>
      <span
        role="tooltip"
        className="pointer-events-none absolute top-6 left-1/2 z-40 hidden w-56 -translate-x-1/2 border border-app-border bg-app-panel p-2 text-left text-xs leading-5 font-normal text-app-text normal-case shadow-app-panel group-focus-within:block group-hover:block"
      >
        {content}
      </span>
    </span>
  );
}
