import {
  formatOpusAllowance,
  formatOpusAllowanceDuration,
  resolveOpusAllowance,
} from "../features/account/components/opus-allowance";
import type { SubscriptionSummaryDto } from "../types";

const translate = (
  key: "opusAllowanceNegative" | "opusAllowanceRefillTime",
  options?: { duration: string },
) => {
  if (key === "opusAllowanceNegative") return "overdrawn";
  return `~${options?.duration ?? ""}`;
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
  it("formats a full allowance without a refill-time suffix", () => {
    expect(formatOpusAllowance({ ...usage, percent: 100 }, translate)).toEqual({
      text: "100%",
      tone: "normal",
    });
  });

  it("formats the remaining allowance and estimated time to refill completely", () => {
    expect(formatOpusAllowance(usage, translate)).toEqual({
      text: "62% · ~83h",
      tone: "normal",
    });
    expect(formatOpusAllowance({ ...usage, percent: 99 }, translate)).toEqual({
      text: "99% · ~2h",
      tone: "normal",
    });
  });

  it.each([
    [1, "1h"],
    [1800, "1h"],
    [5400, "1h"],
    [5401, "2h"],
    [9000, "2h"],
    [9001, "3h"],
    [12600, "3h"],
    [12601, "4h"],
  ])("formats %i seconds as %s with half-hour ties rounded down", (seconds, expected) => {
    expect(formatOpusAllowanceDuration(seconds)).toBe(expected);
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
