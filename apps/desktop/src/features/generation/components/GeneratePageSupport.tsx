import { Pause, Play, Square } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppIconButton, EmptyState } from "@/components/ui";

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
  const { t } = useTranslation("generation");
  const accountLabel = accountError
    ? accountError
    : accountPending
      ? t("account")
      : `${anlasBalance ?? 0} Anlas`;
  const estimateLabel = estimateError
    ? t("estimateUnavailable")
    : estimatePending
      ? t("estimating")
      : t("planned", { count: estimateTotal ?? 0 });

  return (
    <div className="flex items-center gap-2 text-xs text-app-muted">
      <span className="border border-app-border bg-app-panel px-2 py-1">{accountLabel}</span>
      <span className="border border-app-border bg-app-panel px-2 py-1">{estimateLabel}</span>
    </div>
  );
}

export function GenerationLoadingState() {
  const { t } = useTranslation("generation");
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState title={t("loadingDefaults")} />
    </div>
  );
}

export function GenerationSettingsError({ error }: { error: unknown }) {
  const { t } = useTranslation("generation");
  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <EmptyState title={t("settingsUnavailable")} description={formatGenerationError(error)} />
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
    <div className="flex items-center gap-1">
      <AppIconButton
        icon={Pause}
        label="Pause queue"
        size="sm"
        disabled={!canPause || pausePending}
        onClick={onPause}
      />
      <AppIconButton
        icon={Play}
        label="Resume queue"
        size="sm"
        disabled={!canResume || resumePending}
        onClick={onResume}
      />
      <AppIconButton
        icon={Square}
        label="Stop queue"
        size="sm"
        disabled={!canStop || stopPending}
        onClick={onStop}
      />
    </div>
  );
}
