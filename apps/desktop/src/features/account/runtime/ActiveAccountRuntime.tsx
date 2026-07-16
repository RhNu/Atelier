import { useActiveAccountSummaryQuery } from "../data/useActiveAccountSummaryQuery";

export function ActiveAccountRuntime({ enabled }: { enabled: boolean }) {
  useActiveAccountSummaryQuery(enabled);
  return null;
}
