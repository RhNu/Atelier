import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";

import { describeError, frontendLogger } from "./logger";

export function createAtelierQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => {
        frontendLogger.error("Query failed", {
          queryKey: query.queryKey,
          error: describeError(error),
        });
      },
    }),
    mutationCache: new MutationCache({
      onError: (error, _variables, _onMutateResult, mutation) => {
        frontendLogger.error("Mutation failed", {
          mutationKey: mutation.options.mutationKey,
          error: describeError(error),
        });
      },
    }),
    defaultOptions: {
      queries: {
        refetchOnWindowFocus: false,
        retry: 1,
        staleTime: 12_000,
      },
      mutations: {
        retry: 0,
      },
    },
  });
}
