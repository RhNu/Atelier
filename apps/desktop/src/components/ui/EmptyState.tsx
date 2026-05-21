import type { ReactNode } from "react";
import { Inbox } from "lucide-react";

type EmptyStateProps = {
  title: string;
  description?: string;
  action?: ReactNode;
};

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <div className="flex min-h-48 flex-col items-center justify-center border border-dashed border-app-border bg-app-panel/45 px-6 py-10 text-center">
      <Inbox aria-hidden="true" className="mb-4 size-10 text-app-muted/55" />
      <h2 className="text-base font-semibold text-app-text">{title}</h2>
      {description ? <p className="mt-2 max-w-md text-sm text-app-muted">{description}</p> : null}
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}
