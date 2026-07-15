import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import type { SubscriptionSummaryDto } from "@/types";

import { Metric, SectionHeader } from "./SettingsControls";

export function ActiveSubscriptionPanel({
  pending,
  error,
  summary,
  onRefresh,
}: {
  pending: boolean;
  error: string | null;
  summary: SubscriptionSummaryDto | null;
  onRefresh: () => void;
}) {
  const { t } = useTranslation("settings");
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden">
      <SectionHeader
        kicker={t("subscription")}
        title={t("activeKeyProbe")}
        description={t("subscriptionDescription")}
      />
      <div className="grid gap-3 p-3">
        {pending ? <p className="text-sm text-app-muted">{t("checkingSubscription")}</p> : null}
        {error ? <p className="text-sm text-amber-200">{error}</p> : null}
        {summary ? <SubscriptionSummary summary={summary} /> : null}
        {!pending && !error && !summary ? (
          <p className="text-sm text-app-muted">{t("noSubscriptionProbe")}</p>
        ) : null}
        <AppButton variant="secondary" disabled={pending} onClick={onRefresh}>
          <RefreshCw aria-hidden="true" className="size-4" />
          {t("refreshSubscription")}
        </AppButton>
      </div>
    </AppPanel>
  );
}

function SubscriptionSummary({ summary }: { summary: SubscriptionSummaryDto }) {
  const { t, i18n } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  return (
    <dl className="grid gap-2 text-sm">
      <Metric label={t("tier")} value={summary.tier_name} />
      <Metric label="Anlas" value={`${summary.anlas_balance} Anlas`} />
      <Metric
        label={t("opusAccess")}
        value={summary.is_opus ? translateCommon("yes") : translateCommon("no")}
      />
      <Metric
        label={t("expires")}
        value={
          summary.expires_at_ms
            ? new Intl.DateTimeFormat(i18n.resolvedLanguage, {
                dateStyle: "medium",
                timeStyle: "short",
              }).format(new Date(summary.expires_at_ms))
            : translateCommon("unknown")
        }
      />
    </dl>
  );
}
