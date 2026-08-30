import { useMutation, useQuery } from "@tanstack/react-query";

import { appUpdateApi, queryKeys } from "@/platform/atelier";
import type { AppUpdateProgressDto } from "@/platform/atelier";

export function useAppUpdateQuery() {
  return useQuery({
    queryKey: queryKeys.app.appUpdate(),
    queryFn: appUpdateApi.check,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useInstallAppUpdateMutation() {
  return useMutation({
    mutationFn: (onProgress: (progress: AppUpdateProgressDto) => void) =>
      appUpdateApi.install(onProgress),
  });
}
