import type { ResourceRefDto } from "../../../types";

export function GuidancePanelTitle({
  title,
  resource,
}: {
  title: string;
  resource: ResourceRefDto | null;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <h3 className="text-xs font-semibold text-app-muted uppercase">{title}</h3>
      <span className="max-w-44 truncate text-xs text-app-muted">{resource?.id ?? "None"}</span>
    </div>
  );
}
