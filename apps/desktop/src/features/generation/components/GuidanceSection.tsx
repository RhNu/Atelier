import { ChevronDown, Settings2 } from "lucide-react";
import { useCallback, useState, type ReactNode, type SyntheticEvent } from "react";

import { AppHelpMarker } from "../../../components/ui";
import type { ResourceRefDto } from "../../../types";

export function GuidanceSection({
  title,
  help,
  actions,
  children,
}: {
  title: string;
  help?: string;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <section className="grid gap-2 py-1">
      <header className="flex min-h-8 items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-1">
          <h3 className="truncate text-[11px] font-bold tracking-wider text-app-muted uppercase">
            {title}
          </h3>
          {help ? <AppHelpMarker label={`${title} help`} content={help} /> : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-1">{actions}</div> : null}
      </header>
      {children}
    </section>
  );
}

export function GuidanceSettingsDisclosure({
  title = "Settings",
  defaultOpen = false,
  children,
}: {
  title?: string;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const handleToggle = useCallback((event: SyntheticEvent<HTMLDetailsElement>) => {
    setOpen(event.currentTarget.open);
  }, []);

  return (
    <details
      className="group border-t border-app-border/70 pt-2"
      open={open}
      onToggle={handleToggle}
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-2 text-[11px] font-semibold tracking-wide text-app-muted uppercase hover:text-app-text">
        <span className="flex items-center gap-1.5">
          <Settings2 aria-hidden="true" className="size-3.5" />
          {title}
        </span>
        <ChevronDown
          aria-hidden="true"
          className="size-3.5 transition-transform group-open:rotate-180"
        />
      </summary>
      <div className="mt-3 grid gap-3">{children}</div>
    </details>
  );
}

export function GuidanceDeveloperMetadata({
  enabled,
  resource,
  vibeId,
  label,
}: {
  enabled: boolean;
  resource?: ResourceRefDto | null;
  vibeId?: string | null;
  label?: string;
}) {
  if (!enabled) {
    return null;
  }

  return (
    <div className="grid gap-1 border-t border-dashed border-app-border/70 pt-2 font-mono text-[10px] text-app-muted">
      {resource ? (
        <span className="break-all">
          {label ? `${label} ` : ""}resource: {resource.id}
        </span>
      ) : null}
      {resource?.variant_id ? (
        <span className="break-all">variant: {resource.variant_id}</span>
      ) : null}
      {vibeId ? <span className="break-all">vibe: {vibeId}</span> : null}
    </div>
  );
}
