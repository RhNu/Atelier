import { QueryClient } from "@tanstack/react-query";

import { clearWorkspaceScopedQueryCache, isWorkspaceScopedQueryKey } from "./query-cache";
import { queryKeys } from "./query-keys";

describe("workspace query cache", () => {
  it("identifies workspace-scoped command data", () => {
    expect(isWorkspaceScopedQueryKey(queryKeys.workspace.status())).toBe(false);
    expect(isWorkspaceScopedQueryKey(queryKeys.gallery.root())).toBe(true);
    expect(isWorkspaceScopedQueryKey(queryKeys.prompt.chunks())).toBe(true);
  });

  it("clears workspace-scoped data while keeping boot state", async () => {
    const queryClient = new QueryClient();

    queryClient.setQueryData(queryKeys.workspace.status(), {
      root: "D:/atelier-a",
      schema_version: 4,
      locked: false,
    });
    queryClient.setQueryData(queryKeys.gallery.root(), { items: [] });
    queryClient.setQueryData(queryKeys.settings.workspace(), { generation: {} });

    await clearWorkspaceScopedQueryCache(queryClient);

    expect(queryClient.getQueryData(queryKeys.workspace.status())).toEqual({
      root: "D:/atelier-a",
      schema_version: 4,
      locked: false,
    });
    expect(queryClient.getQueryData(queryKeys.gallery.root())).toBeUndefined();
    expect(queryClient.getQueryData(queryKeys.settings.workspace())).toBeUndefined();
  });
});
