import { Pause, Play, Square } from "lucide-react";

import { AppIconButton, EmptyState } from "../../../components/ui";
import { formatGenerationError } from "../generation-page-utils";

type QueueControlsProps = {
  canPause: boolean;
  canResume: boolean;
  canStop: boolean;
  pausePending: boolean;
  resumePending: boolean;
  stopPending: boolean;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
};

export function GenerationEconomyStatus({
  accountPending,
  accountError,
  anlasBalance,
  estimatePending,
  estimateError,
  estimateTotal,
}: {
  accountPending: boolean;
  accountError: string | null;
  anlasBalance: number | null;
  estimatePending: boolean;
  estimateError: string | null;
  estimateTotal: number | null;
}) {
  const accountLabel = accountError
    ? accountError
    : accountPending
      ? "Account"
      : `${anlasBalance ?? 0} Anlas`;
  const estimateLabel = estimateError
    ? "Estimate unavailable"
    : estimatePending
      ? "Estimating"
      : `${estimateTotal ?? 0} planned`;

  return (
    <div className="flex items-center gap-2 text-xs text-app-muted">
      <span className="border border-app-border bg-app-panel px-2 py-1">{accountLabel}</span>
      <span className="border border-app-border bg-app-panel px-2 py-1">{estimateLabel}</span>
    </div>
  );
}

export function GenerationLoadingState() {
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState title="Loading generation defaults" />
    </div>
  );
}

export function GenerationSettingsError({ error }: { error: unknown }) {
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState
        title="Generation settings unavailable"
        description={formatGenerationError(error)}
      />
    </div>
  );
}

export function QueueControls({
  canPause,
  canResume,
  canStop,
  pausePending,
  resumePending,
  stopPending,
  onPause,
  onResume,
  onStop,
}: QueueControlsProps) {
  return (
    <div className="flex items-center gap-2">
      <AppIconButton
        icon={Pause}
        label="Pause queue"
        disabled={!canPause || pausePending}
        onClick={onPause}
      />
      <AppIconButton
        icon={Play}
        label="Resume queue"
        disabled={!canResume || resumePending}
        onClick={onResume}
      />
      <AppIconButton
        icon={Square}
        label="Stop queue"
        disabled={!canStop || stopPending}
        onClick={onStop}
      />
    </div>
  );
}
