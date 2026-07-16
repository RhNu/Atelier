import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { accountApi, queryKeys } from "@/platform/atelier";
import type {
  CreateApiKeyRequestDto,
  DeleteApiKeyRequestDto,
  SetActiveApiKeyRequestDto,
  UpdateApiKeyRequestDto,
} from "@/types";

function useRefreshAccountKeys() {
  const queryClient = useQueryClient();

  return async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.account.apiKeys() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.account.activeSummary() }),
    ]);
  };
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
    mutationFn: (request: CreateApiKeyRequestDto) => accountApi.create(request),
    onSuccess: refreshAccountKeys,
  });
}

export function useUpdateApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: UpdateApiKeyRequestDto) => accountApi.update(request),
    onSuccess: async () => {
      await refreshAccountKeys();
    },
  });
}

export function useDeleteApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: DeleteApiKeyRequestDto) => accountApi.delete(request),
    onSuccess: async () => {
      await refreshAccountKeys();
    },
  });
}

export function useSetActiveApiKeyMutation() {
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: SetActiveApiKeyRequestDto) => accountApi.setActive(request),
    onSuccess: refreshAccountKeys,
  });
}
