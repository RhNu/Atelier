import type { QueryClient, QueryKey } from "@tanstack/react-query";

const workspaceIndependentRoots = new Set(["workspace"]);

export function isWorkspaceScopedQueryKey(queryKey: QueryKey): boolean {
  const [root] = queryKey;
  return typeof root === "string" && !workspaceIndependentRoots.has(root);
}

export async function clearWorkspaceScopedQueryCache(queryClient: QueryClient): Promise<void> {
  const predicate = (query: { queryKey: QueryKey }) => isWorkspaceScopedQueryKey(query.queryKey);

  await queryClient.cancelQueries({ predicate });
  queryClient.removeQueries({ predicate });
}
