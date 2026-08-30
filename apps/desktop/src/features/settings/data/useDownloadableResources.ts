import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { downloadableResourcesApi, queryKeys } from "@/platform/atelier";
import type {
  DownloadableResourceGroupRequestDto,
  DownloadableResourceInstallProgressDto,
  DownloadableResourceRequestDto,
} from "@/types";

export function useDownloadableResourcesQuery() {
  return useQuery({
    queryKey: queryKeys.app.downloadableResources(),
    queryFn: downloadableResourcesApi.list,
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.resources.some((resource) =>
        ["downloading", "verifying"].includes(resource.state),
      )
        ? 1_000
        : false,
  });
}

export function useRefreshDownloadableResourcesMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: downloadableResourcesApi.refresh,
    onSuccess: (data) => queryClient.setQueryData(queryKeys.app.downloadableResources(), data),
  });
}

export function useInstallDownloadableResourceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      request,
      onProgress,
    }: {
      request: DownloadableResourceRequestDto;
      onProgress: (progress: DownloadableResourceInstallProgressDto) => void;
    }) => downloadableResourcesApi.install(request, onProgress),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.downloadableResources() }),
  });
}

export function useInstallDownloadableResourceGroupMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      request,
      onProgress,
    }: {
      request: DownloadableResourceGroupRequestDto;
      onProgress: (progress: DownloadableResourceInstallProgressDto) => void;
    }) => downloadableResourcesApi.installGroup(request, onProgress),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.downloadableResources() }),
  });
}

export function useCancelDownloadableResourceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: downloadableResourcesApi.cancelInstall,
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.downloadableResources() }),
  });
}

export function useDeleteDownloadableResourceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: downloadableResourcesApi.delete,
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.downloadableResources() }),
  });
}

export function useCompleteResourceOnboardingMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: downloadableResourcesApi.completeOnboarding,
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.downloadableResources() }),
  });
}
