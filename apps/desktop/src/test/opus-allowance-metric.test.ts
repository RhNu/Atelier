import {
  formatOpusAllowance,
  formatOpusAllowanceDuration,
  resolveOpusAllowance,
} from "../features/account/components/opus-allowance";
import type { SubscriptionSummaryDto } from "../types";

const translate = (
  key: "opusAllowanceNegative" | "opusAllowanceRate",
  options?: { duration: string },
) => {
  if (key === "opusAllowanceNegative") return "overdrawn";
  return `+1%/${options?.duration ?? ""}`;
};

const usage = {
  is_negative: false,
  percent: 62,
  seconds_until_next_percent: 7888,
};

const summary = {
  anlas_balance: 10_000,
  is_opus: true,
  subscription_active: true,
  tier: 3,
  tier_name: "Opus",
  expires_at_ms: null,
  v5_usage: usage,
} satisfies SubscriptionSummaryDto;

describe("OpusAllowanceMetric", () => {
  it("formats a full allowance without a refill-rate suffix", () => {
    expect(formatOpusAllowance({ ...usage, percent: 100 }, translate)).toEqual({
      text: "100%",
      tone: "normal",
    });
  });

  it("formats the remaining allowance and refill rate", () => {
    expect(formatOpusAllowance(usage, translate)).toEqual({
      text: "62% · +1%/2h11m",
      tone: "normal",
    });
    expect(formatOpusAllowanceDuration(7888)).toBe("2h11m");
  });

  it("formats an overdrawn allowance with warning tone", () => {
    expect(formatOpusAllowance({ ...usage, is_negative: true }, translate)).toEqual({
      text: "overdrawn",
      tone: "warning",
    });
  });

  it("requires an active Opus account and the selected model's usage-pool capability", () => {
    const capabilities = { has_opus_usage_limit: true };
    expect(resolveOpusAllowance(summary)).toEqual(usage);
    expect(resolveOpusAllowance(summary, capabilities)).toEqual(usage);
    expect(
      resolveOpusAllowance(summary, { ...capabilities, has_opus_usage_limit: false }),
    ).toBeNull();
    expect(resolveOpusAllowance({ ...summary, is_opus: false }, capabilities)).toBeNull();
  });
});
