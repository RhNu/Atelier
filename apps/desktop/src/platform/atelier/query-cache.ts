import type { QueryClient, QueryKey } from "@tanstack/react-query";

export function isWorkspaceScopedQueryKey(queryKey: QueryKey): boolean {
  const [root] = queryKey;
  return root === "workspace";
}

export async function clearWorkspaceScopedQueryCache(queryClient: QueryClient): Promise<void> {
  const predicate = (query: { queryKey: QueryKey }) => isWorkspaceScopedQueryKey(query.queryKey);

  await queryClient.cancelQueries({ predicate });
  queryClient.removeQueries({ predicate });
}
