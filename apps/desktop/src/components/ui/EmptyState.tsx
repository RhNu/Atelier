import { Inbox } from "lucide-react";
import type { ReactNode } from "react";

type EmptyStateProps = {
  title: string;
  description?: string;
  action?: ReactNode;
  iconOnly?: boolean;
};

export function EmptyState({ title, description, action, iconOnly = false }: EmptyStateProps) {
  return (
    <div
      role={iconOnly ? "img" : undefined}
      aria-label={iconOnly ? title : undefined}
      className="flex min-h-48 flex-col items-center justify-center border border-dashed border-app-border bg-app-panel/45 px-6 py-10 text-center"
    >
      <Inbox
        aria-hidden="true"
        className={["size-10 text-app-muted/55", iconOnly ? "" : "mb-4"].join(" ")}
      />
      {iconOnly ? null : <h2 className="text-base font-semibold text-app-text">{title}</h2>}
      {!iconOnly && description ? (
        <p className="mt-2 max-w-md text-sm text-app-muted">{description}</p>
      ) : null}
      {!iconOnly && action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}
