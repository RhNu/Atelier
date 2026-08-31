import { useInfiniteQuery, useMutation, useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { runLoggedAction } from "@/app/logger";
import {
  desktopApi,
  exploreApi,
  globalSettingsApi,
  lexiconApi,
  queryKeys,
  resourceImageToDataUrl,
} from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type { ExploreItemRefDto, ExploreMediaVariantDto, ExploreQueryDto } from "@/types";

import { useExploreIdentity } from "../state/explore-identity";

export function useExploreSources() {
  return useQuery({
    queryKey: queryKeys.explore.sources(),
    queryFn: exploreApi.sources,
    staleTime: Infinity,
  });
}

export function useExploreSearchQuery(query: ExploreQueryDto | null, active: boolean) {
  const revision = useExploreIdentity((state) =>
    query?.source_id === "danbooru_database" ? state.revision : 0,
  );
  return useInfiniteQuery({
    queryKey: queryKeys.explore.search(query, revision),
    queryFn: ({ pageParam }) => {
      if (!query) throw new Error("Explore query is required");
      return exploreApi.search({ query, cursor: pageParam });
    },
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.next_cursor ?? undefined,
    enabled: active && query !== null,
    staleTime: 120_000,
    gcTime: 10 * 60_000,
    retry: false,
    refetchOnReconnect: false,
  });
}

export function useExploreDetailQuery(item: ExploreItemRefDto | null, active: boolean) {
  const revision = useExploreIdentity((state) =>
    item?.source_id === "danbooru_database" ? state.revision : 0,
  );
  return useQuery({
    queryKey: queryKeys.explore.detail(item, revision),
    queryFn: () => {
      if (!item) throw new Error("Explore item is required");
      return exploreApi.detail(item);
    },
    enabled: active && item !== null,
    staleTime: 120_000,
    retry: false,
  });
}

export function useExploreMediaQuery(item: ExploreItemRefDto, variant: ExploreMediaVariantDto) {
  const revision = useExploreIdentity((state) =>
    item.source_id === "danbooru_database" ? state.revision : 0,
  );
  return useQuery({
    queryKey: queryKeys.explore.media(item, variant, revision),
    queryFn: async () => resourceImageToDataUrl(await exploreApi.media({ item, variant })),
    staleTime: 120_000,
    gcTime: 30_000,
    retry: false,
    refetchOnReconnect: false,
  });
}

export function useExploreSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.app.globalSettings(),
    queryFn: () => globalSettingsApi.get(),
  });
}

export function useCopyExploreText() {
  const { t } = useTranslation("explore");
  const pushToast = useToastStore((state) => state.push);
  return useMutation({
    mutationFn: (text: string) =>
      runLoggedAction("Copy Explore text", () => desktopApi.copyText(text)),
    onSuccess: () =>
      pushToast({ level: "success", title: t("copySucceeded"), message: t("copied") }),
    onError: (error) =>
      pushToast({ level: "error", title: t("copyFailed"), message: String(error) }),
  });
}

export function useExploreTagCompletion(query: string, active = true) {
  return useQuery({
    queryKey: queryKeys.lexicon.completion(query, 8),
    queryFn: () => lexiconApi.complete({ query, limit: 8 }),
    enabled: active && query.length >= 2 && !query.includes(":"),
    staleTime: Number.POSITIVE_INFINITY,
  });
}
