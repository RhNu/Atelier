import type { ModelCapabilitiesDto, SubscriptionSummaryDto, V5UsageStatusDto } from "@/types";

type OpusAllowanceTranslationKey = "opusAllowanceNegative" | "opusAllowanceRate";
export type TranslateOpusAllowance = (
  key: OpusAllowanceTranslationKey,
  options?: { duration: string },
) => string;

export type FormattedOpusAllowance = {
  text: string;
  tone: "normal" | "warning";
};

type OpusUsageCapabilities = Pick<ModelCapabilitiesDto, "has_opus_usage_limit">;

export function resolveOpusAllowance(
  summary: SubscriptionSummaryDto | null | undefined,
): V5UsageStatusDto | null;
export function resolveOpusAllowance(
  summary: SubscriptionSummaryDto | null | undefined,
  capabilities: OpusUsageCapabilities | undefined,
): V5UsageStatusDto | null;
export function resolveOpusAllowance(
  summary: SubscriptionSummaryDto | null | undefined,
  capabilities?: OpusUsageCapabilities,
): V5UsageStatusDto | null {
  if (!summary?.is_opus || !summary.subscription_active || !summary.v5_usage) return null;
  if (arguments.length >= 2 && capabilities?.has_opus_usage_limit !== true) return null;
  return summary.v5_usage;
}

export function formatOpusAllowance(
  usage: V5UsageStatusDto,
  translate: TranslateOpusAllowance,
): FormattedOpusAllowance {
  if (usage.is_negative) {
    return { text: translate("opusAllowanceNegative"), tone: "warning" };
  }
  if (usage.percent >= 100 || usage.seconds_until_next_percent <= 0) {
    return { text: `${usage.percent}%`, tone: "normal" };
  }
  const duration = formatOpusAllowanceDuration(usage.seconds_until_next_percent);
  return {
    text: `${usage.percent}% · ${translate("opusAllowanceRate", { duration })}`,
    tone: "normal",
  };
}

export function formatOpusAllowanceDuration(seconds: number): string {
  const totalMinutes = Math.max(0, Math.floor(seconds / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours > 0 ? `${hours}h${minutes}m` : `${minutes}m`;
}
