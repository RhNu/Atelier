import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { imageAnalysisApi, queryKeys } from "@/platform/atelier";
import type { ImageAnalysisModelInstallProgressDto, ImageAnalysisModelRequestDto } from "@/types";

export function useImageAnalysisModelsQuery() {
  return useQuery({
    queryKey: queryKeys.app.imageAnalysisModels(),
    queryFn: imageAnalysisApi.statuses,
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.some((model) => model.state === "installing") ? 1_000 : false,
  });
}

export function useInstallImageAnalysisModelMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      request,
      onProgress,
    }: {
      request: ImageAnalysisModelRequestDto;
      onProgress: (progress: ImageAnalysisModelInstallProgressDto) => void;
    }) => imageAnalysisApi.install(request, onProgress),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.imageAnalysisModels() }),
  });
}

export function useCancelImageAnalysisModelInstallMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: imageAnalysisApi.cancelInstall,
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.imageAnalysisModels() }),
  });
}

export function useDeleteImageAnalysisModelMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: imageAnalysisApi.delete,
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: queryKeys.app.imageAnalysisModels() }),
  });
}
