import { useActiveAccountSummaryQuery } from "@/features/account/data/useActiveAccountSummaryQuery";

export function useDirectorReadinessQuery() {
  return useActiveAccountSummaryQuery();
}
