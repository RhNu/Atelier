import { useQuery } from "@tanstack/react-query";

import { accountApi, queryKeys } from "@/platform/atelier";

export function useActiveAccountSummaryQuery(enabled = true) {
  return useQuery({
    queryKey: queryKeys.account.activeSummary(),
    queryFn: () => accountApi.probeActive(),
    enabled,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
}
