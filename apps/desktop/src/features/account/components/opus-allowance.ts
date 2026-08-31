import type { ModelCapabilitiesDto, SubscriptionSummaryDto, V5UsageStatusDto } from "@/types";

type OpusAllowanceTranslationKey = "opusAllowanceNegative" | "opusAllowanceRefillTime";
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
  const duration = formatOpusAllowanceDuration(
    (100 - usage.percent) * usage.seconds_until_next_percent,
  );
  return {
    text: `${usage.percent}% · ${translate("opusAllowanceRefillTime", { duration })}`,
    tone: "normal",
  };
}

export function formatOpusAllowanceDuration(seconds: number): string {
  // Half-hour boundaries round down, with a minimum display of one hour.
  const hours = Math.max(1, Math.ceil(seconds / 3600 - 0.5));
  return `${hours}h`;
}
