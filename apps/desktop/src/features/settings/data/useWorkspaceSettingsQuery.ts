import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { queryKeys, settingsApi } from "../../../platform/atelier";
import type { UpdateWorkspaceSettingsRequestDto } from "../../../types";

export function useWorkspaceSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}

export function useUpdateWorkspaceSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateWorkspaceSettingsRequestDto) => settingsApi.update(request),
    onSuccess: async (settings) => {
      queryClient.setQueryData(queryKeys.settings.workspace(), settings);
      await queryClient.invalidateQueries({ queryKey: queryKeys.settings.workspace() });
    },
  });
}

export function useResetWorkspaceSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => settingsApi.reset(),
    onSuccess: async (response) => {
      queryClient.setQueryData(queryKeys.settings.workspace(), response.settings);
      await queryClient.invalidateQueries({ queryKey: queryKeys.settings.workspace() });
    },
  });
}
