import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { globalSettingsApi, queryKeys } from "../../../platform/atelier";
import type { UpdateGlobalSettingsRequestDto } from "../../../types";

export function useGlobalSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.app.globalSettings(),
    queryFn: () => globalSettingsApi.get(),
  });
}

export function useUpdateGlobalSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateGlobalSettingsRequestDto) => globalSettingsApi.update(request),
    onSuccess: (settings) => {
      queryClient.setQueryData(queryKeys.app.globalSettings(), settings);
      queryClient.setQueryData(queryKeys.app.bootstrap(), (current: unknown) => {
        if (!current || typeof current !== "object") {
          return current;
        }
        return { ...current, global_settings: settings };
      });
    },
  });
}
