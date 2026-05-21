import { QueryClient } from "@tanstack/react-query";

export function createAtelierQueryClient(): QueryClient {
  return new QueryClient({
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
