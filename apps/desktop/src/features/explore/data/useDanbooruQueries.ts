import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";

import { danbooruApi, queryKeys } from "@/platform/atelier";
import type { DanbooruExploreQueryDto, SaveDanbooruAccountRequestDto } from "@/types";

import { useExploreIdentity } from "../state/explore-identity";
import { useExploreSearchQuery, useExploreDetailQuery } from "./useExploreQueries";

export function useDanbooruSearchQuery(request: DanbooruExploreQueryDto | null, active: boolean) {
  const query = useExploreSearchQuery(
    request ? { source_id: "danbooru_database", query: request } : null,
    active,
  );
  const data = useMemo(
    () =>
      query.data
        ? {
            ...query.data,
            pages: query.data.pages.map((page) => ({
              ...page,
              items: page.items.flatMap((item) =>
                item.source_id === "danbooru_database" ? [item.post] : [],
              ),
            })),
          }
        : undefined,
    [query.data],
  );
  return { ...query, data };
}

export function useDanbooruDetailQuery(postId: number | null, active: boolean) {
  const query = useExploreDetailQuery(
    postId === null ? null : { source_id: "danbooru_database", item_id: String(postId) },
    active,
  );
  return {
    ...query,
    data: query.data?.source_id === "danbooru_database" ? query.data.detail : undefined,
  };
}

export function useDanbooruAccountQuery(active = true) {
  return useQuery({
    enabled: active,
    queryKey: queryKeys.danbooru.account(),
    queryFn: async () => toStoredDanbooruAccount(await danbooruApi.account()),
  });
}

export function useDanbooruAccountMutations() {
  const queryClient = useQueryClient();
  const update = (account: Awaited<ReturnType<typeof danbooruApi.account>>) => {
    useExploreIdentity.getState().advance();
    queryClient.removeQueries({ queryKey: queryKeys.explore.source("danbooru_database") });
    queryClient.setQueryData(queryKeys.danbooru.account(), toStoredDanbooruAccount(account));
    void queryClient.invalidateQueries({ queryKey: queryKeys.danbooru.root() });
  };
  return {
    save: useMutation({
      mutationFn: (request: SaveDanbooruAccountRequestDto) => danbooruApi.saveAccount(request),
      onSuccess: update,
    }),
    probe: useMutation({
      mutationFn: () => danbooruApi.probeAccount(),
    }),
    remove: useMutation({
      mutationFn: () => danbooruApi.deleteAccount(),
      onSuccess: update,
    }),
  };
}

function toStoredDanbooruAccount(account: Awaited<ReturnType<typeof danbooruApi.account>>) {
  return { configured: account.configured, username: account.username };
}
