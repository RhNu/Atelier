import { CircleHelp } from "lucide-react";

export function AppHelpMarker({
  content,
  label = "Help",
  hoverOnly = false,
}: {
  content: string;
  label?: string;
  hoverOnly?: boolean;
}) {
  return (
    <span className="group relative inline-flex shrink-0">
      {hoverOnly ? (
        <span aria-label={label} className="grid size-5 place-items-center text-app-muted">
          <CircleHelp aria-hidden="true" className="size-3.5" />
        </span>
      ) : (
        <button
          type="button"
          aria-label={label}
          className="grid size-5 place-items-center text-app-muted outline-none hover:text-app-text focus-visible:text-app-text"
        >
          <CircleHelp aria-hidden="true" className="size-3.5" />
        </button>
      )}
      <span
        role="tooltip"
        className={[
          "pointer-events-none absolute top-6 left-1/2 z-40 hidden w-56 -translate-x-1/2 border border-app-border bg-app-panel p-2 text-left text-xs leading-5 font-normal text-app-text normal-case shadow-app-panel group-hover:block",
          hoverOnly ? "" : "group-focus-within:block",
        ].join(" ")}
      >
        {content}
      </span>
    </span>
  );
}
