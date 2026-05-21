import { useQuery } from "@tanstack/react-query";

import { accountApi, queryKeys } from "../../../platform/atelier";

export function useDirectorReadinessQuery() {
  return useQuery({
    queryKey: queryKeys.account.activeProbe(),
    queryFn: () => accountApi.probeActive(),
    retry: 0,
  });
}
