import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { runLoggedAction } from "@/app/logger";
import {
  danbooruApi,
  desktopApi,
  globalSettingsApi,
  lexiconApi,
  queryKeys,
  resourceImageToDataUrl,
} from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type {
  DanbooruMediaVariantDto,
  DanbooruSearchRequestDto,
  SaveDanbooruAccountRequestDto,
} from "@/types";

export function useCopyDanbooruTagsMutation() {
  const { t } = useTranslation("inspiration");
  const pushToast = useToastStore((state) => state.push);
  return useMutation({
    mutationFn: (text: string) =>
      runLoggedAction("Copy Danbooru tags", () => desktopApi.copyText(text)),
    onSuccess: () =>
      pushToast({ level: "success", message: t("copied"), title: t("copySucceeded") }),
    onError: (error) =>
      pushToast({ level: "error", title: t("copyFailed"), message: String(error) }),
  });
}

export function useDanbooruSearchQuery(
  request: Omit<DanbooruSearchRequestDto, "before_id"> | null,
) {
  return useInfiniteQuery({
    queryKey: request
      ? queryKeys.danbooru.search(request)
      : (["app", "danbooru", "search", null] as const),
    queryFn: ({ pageParam }) => {
      if (!request) throw new Error("Danbooru search request is required");
      return danbooruApi.search({ ...request, before_id: pageParam });
    },
    initialPageParam: null as number | null,
    getNextPageParam: (page) => page.next_before_id ?? undefined,
    enabled: request !== null,
    staleTime: 120_000,
    retry: false,
  });
}

export function useDanbooruDetailQuery(postId: number | null) {
  return useQuery({
    queryKey: queryKeys.danbooru.detail(postId),
    queryFn: () => {
      if (postId === null) throw new Error("Danbooru post ID is required");
      return danbooruApi.detail({ post_id: postId });
    },
    enabled: postId !== null,
    staleTime: 30 * 60_000,
    retry: false,
  });
}

export function useDanbooruMediaQuery(
  postId: number,
  variant: DanbooruMediaVariantDto,
  enabled: boolean,
) {
  return useQuery({
    queryKey: queryKeys.danbooru.media(postId, variant),
    queryFn: async () =>
      resourceImageToDataUrl(await danbooruApi.media({ post_id: postId, variant })),
    enabled,
    staleTime: 30 * 60_000,
    gcTime: 60 * 60_000,
    retry: false,
  });
}

export function useDanbooruAccountQuery() {
  return useQuery({
    queryKey: queryKeys.danbooru.account(),
    queryFn: async () => toStoredDanbooruAccount(await danbooruApi.account()),
  });
}

export function useDanbooruGlobalSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.app.globalSettings(),
    queryFn: () => globalSettingsApi.get(),
  });
}

export function useDanbooruAccountMutations() {
  const queryClient = useQueryClient();
  const update = (account: Awaited<ReturnType<typeof danbooruApi.account>>) => {
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

export function useDanbooruTagCompletion(query: string) {
  return useQuery({
    queryKey: queryKeys.lexicon.completion(query, 8),
    queryFn: () => lexiconApi.complete({ query, limit: 8 }),
    enabled: query.length >= 2 && !query.includes(":"),
    staleTime: Number.POSITIVE_INFINITY,
  });
}
