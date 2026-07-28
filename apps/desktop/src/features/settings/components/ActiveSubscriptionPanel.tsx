import { Loader2, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { SubscriptionSummaryDto } from "@/types";

import { formatError } from "../settings-utils";

export function ActiveSubscriptionPanel({
  pending,
  refreshing,
  missingActiveKey,
  error,
  summary,
  onRetry,
}: {
  pending: boolean;
  refreshing: boolean;
  missingActiveKey: boolean;
  error: unknown;
  summary: SubscriptionSummaryDto | null;
  onRetry: () => void;
}) {
  const { t } = useTranslation("settings");
  const status = missingActiveKey
    ? t("noActiveApiKey")
    : error
      ? formatError(error)
      : pending
        ? t("checkingSubscription")
        : null;
  return (
    <section className="mb-3 border border-app-border bg-app-surface/45">
      <header className="flex min-h-10 items-center justify-between gap-3 border-b border-app-border px-3 py-2">
        <div className="flex min-w-0 items-center gap-3">
          <h3 className="shrink-0 text-sm font-semibold text-app-text">
            {t("activeSubscription")}
          </h3>
          {status ? <span className="truncate text-xs text-app-muted">{status}</span> : null}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {refreshing ? (
            <Loader2
              aria-label={t("refreshingSubscription")}
              className="size-4 animate-spin text-app-muted"
            />
          ) : null}
          {error && !missingActiveKey ? (
            <AppButton variant="ghost" className="h-8 px-2 text-xs" onClick={onRetry}>
              <RotateCcw aria-hidden="true" className="size-3.5" />
              {t("retrySubscription")}
            </AppButton>
          ) : null}
        </div>
      </header>
      <SubscriptionSummary
        summary={summary}
        placeholder={pending || missingActiveKey || Boolean(error)}
      />
    </section>
  );
}

function SubscriptionSummary({
  summary,
  placeholder,
}: {
  summary: SubscriptionSummaryDto | null;
  placeholder: boolean;
}) {
  const { t, i18n } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  return (
    <dl className="flex flex-wrap items-center gap-x-6 gap-y-1 px-3 py-2 text-sm">
      <CompactMetric label={t("tier")} value={formatTier(summary?.tier_name) ?? "—"} />
      <CompactMetric label="Anlas" value={summary ? `${summary.anlas_balance} Anlas` : "—"} />
      <CompactMetric
        label={t("expires")}
        value={
          summary?.expires_at_ms
            ? new Intl.DateTimeFormat(i18n.resolvedLanguage, {
                dateStyle: "medium",
                timeStyle: "short",
              }).format(new Date(summary.expires_at_ms))
            : placeholder
              ? "—"
              : translateCommon("unknown")
        }
      />
    </dl>
  );
}

function CompactMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-baseline gap-2">
      <dt className="text-[10px] text-app-muted uppercase">{label}</dt>
      <dd className="truncate text-sm font-semibold text-app-text">{value}</dd>
    </div>
  );
}

function formatTier(tier: string | undefined): string | undefined {
  if (!tier) return undefined;
  return tier.charAt(0).toUpperCase() + tier.slice(1);
}
