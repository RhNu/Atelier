import { Loader2, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { SubscriptionSummaryDto } from "@/types";

import { formatError } from "../settings-utils";
import { Metric, SectionHeader } from "./SettingsControls";

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
  return (
    <section className="mb-3 border border-app-border bg-app-surface/45">
      <SectionHeader
        kicker={t("subscription")}
        title={t("activeSubscription")}
        description={t("subscriptionDescription")}
      >
        {refreshing ? (
          <Loader2
            aria-label={t("refreshingSubscription")}
            className="size-4 animate-spin text-app-muted"
          />
        ) : null}
      </SectionHeader>
      <div className="grid gap-3 p-3">
        <SubscriptionSummary
          summary={summary}
          placeholder={pending || missingActiveKey || Boolean(error)}
        />
        <div className="flex min-h-8 items-center justify-between gap-3 text-xs text-app-muted">
          <span>
            {missingActiveKey
              ? t("noActiveApiKey")
              : error
                ? formatError(error)
                : pending
                  ? t("checkingSubscription")
                  : t("subscriptionUpToDate")}
          </span>
          {error && !missingActiveKey ? (
            <AppButton variant="ghost" className="h-8 px-2 text-xs" onClick={onRetry}>
              <RotateCcw aria-hidden="true" className="size-3.5" />
              {t("retrySubscription")}
            </AppButton>
          ) : null}
        </div>
      </div>
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
    <dl className="grid gap-2 text-sm">
      <Metric label={t("tier")} value={summary?.tier_name ?? "—"} />
      <Metric label="Anlas" value={summary ? `${summary.anlas_balance} Anlas` : "—"} />
      <Metric
        label={t("opusAccess")}
        value={summary ? (summary.is_opus ? translateCommon("yes") : translateCommon("no")) : "—"}
      />
      <Metric
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
