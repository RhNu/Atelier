import { RefreshCw } from "lucide-react";

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
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden">
      <SectionHeader
        kicker="Subscription"
        title="Active Key Probe"
        description="Refresh the active NovelAI key before generation work."
      />
      <div className="grid gap-3 p-3">
        {pending ? <p className="text-sm text-app-muted">Checking active subscription</p> : null}
        {error ? <p className="text-sm text-amber-200">{error}</p> : null}
        {summary ? <SubscriptionSummary summary={summary} /> : null}
        {!pending && !error && !summary ? (
          <p className="text-sm text-app-muted">No active subscription probe yet.</p>
        ) : null}
        <AppButton variant="secondary" disabled={pending} onClick={onRefresh}>
          <RefreshCw aria-hidden="true" className="size-4" />
          Refresh active subscription
        </AppButton>
      </div>
    </AppPanel>
  );
}

function SubscriptionSummary({ summary }: { summary: SubscriptionSummaryDto }) {
  return (
    <dl className="grid gap-2 text-sm">
      <Metric label="Tier" value={summary.tier_name} />
      <Metric label="Anlas" value={`${summary.anlas_balance} Anlas`} />
      <Metric label="Opus access" value={summary.is_opus ? "Yes" : "No"} />
      <Metric
        label="Expires"
        value={summary.expires_at_ms ? new Date(summary.expires_at_ms).toLocaleString() : "Unknown"}
      />
    </dl>
  );
}
