import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { accountApi, queryKeys } from "../../../platform/atelier";
import type {
  CreateApiKeyRequestDto,
  DeleteApiKeyRequestDto,
  ProbeApiKeyRequestDto,
  SetActiveApiKeyRequestDto,
  UpdateApiKeyRequestDto,
} from "../../../types";

function useRefreshAccountKeys() {
  const queryClient = useQueryClient();

  return async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.account.apiKeys() });
    queryClient.removeQueries({ queryKey: queryKeys.account.activeProbe() });
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
  const queryClient = useQueryClient();
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: UpdateApiKeyRequestDto) => accountApi.update(request),
    onSuccess: async (_, request) => {
      queryClient.removeQueries({ queryKey: queryKeys.account.keyProbe(request.id) });
      await refreshAccountKeys();
    },
  });
}

export function useDeleteApiKeyMutation() {
  const queryClient = useQueryClient();
  const refreshAccountKeys = useRefreshAccountKeys();

  return useMutation({
    mutationFn: (request: DeleteApiKeyRequestDto) => accountApi.delete(request),
    onSuccess: async (_, request) => {
      queryClient.removeQueries({ queryKey: queryKeys.account.keyProbe(request.id) });
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

export function useProbeApiKeyMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: ProbeApiKeyRequestDto) => accountApi.probe(request),
    onSuccess: (subscription, request) => {
      queryClient.setQueryData(queryKeys.account.keyProbe(request.id), subscription);
      void queryClient.invalidateQueries({ queryKey: queryKeys.account.activeProbe() });
    },
  });
}

export function useProbeActiveApiKeyMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => accountApi.probeActive(),
    onSuccess: (subscription) => {
      queryClient.setQueryData(queryKeys.account.activeProbe(), subscription);
    },
  });
}
