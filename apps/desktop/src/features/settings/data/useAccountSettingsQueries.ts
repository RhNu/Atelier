import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { runLoggedAction } from "@/app/logger";
import { accountApi, queryKeys } from "@/platform/atelier";
import type {
  CreateApiKeyRequestDto,
  DeleteApiKeyRequestDto,
  SetActiveApiKeyRequestDto,
  UpdateApiKeyRequestDto,
} from "@/types";

function useRefreshAccountKeys() {
  const queryClient = useQueryClient();

  return () =>
    runLoggedAction("Refresh account settings", () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.account.apiKeys() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.account.activeSummary() }),
      ]),
    );
}

export function useApiKeysQuery() {
  return useQuery({
    queryKey: queryKeys.account.apiKeys(),
    queryFn: () => accountApi.list(),
    retry: false,
    refetchOnWindowFocus: false,
  });
}

export function useCreateApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: CreateApiKeyRequestDto) =>
      runLoggedAction("Create NovelAI API key", () => accountApi.create(request)),
    onSuccess: refreshAccountKeys,
  });
}

export function useUpdateApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: UpdateApiKeyRequestDto) =>
      runLoggedAction("Update NovelAI API key", () => accountApi.update(request)),
    onSuccess: async () => {
      await refreshAccountKeys();
    },
  });
}

export function useDeleteApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: DeleteApiKeyRequestDto) =>
      runLoggedAction("Delete NovelAI API key", () => accountApi.delete(request)),
    onSuccess: async () => {
      await refreshAccountKeys();
    },
  });
}

export function useSetActiveApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: SetActiveApiKeyRequestDto) =>
      runLoggedAction("Set active NovelAI API key", () => accountApi.setActive(request)),
    onSuccess: refreshAccountKeys,
  });
}
