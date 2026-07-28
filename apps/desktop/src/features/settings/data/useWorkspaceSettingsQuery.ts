import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { runLoggedAction } from "@/app/logger";
import { queryKeys, settingsApi } from "@/platform/atelier";
import type { UpdateWorkspaceSettingsRequestDto } from "@/types";

export function useWorkspaceSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}

export function useUpdateWorkspaceSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateWorkspaceSettingsRequestDto) =>
      runLoggedAction("Update workspace settings", () => settingsApi.update(request)),
    onSuccess: async (settings) => {
      queryClient.setQueryData(queryKeys.settings.workspace(), settings);
      await runLoggedAction("Refresh workspace settings", () =>
        queryClient.invalidateQueries({ queryKey: queryKeys.settings.workspace() }),
      );
    },
  });
}

export function useResetWorkspaceSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => runLoggedAction("Reset workspace settings", () => settingsApi.reset()),
    onSuccess: async (response) => {
      queryClient.setQueryData(queryKeys.settings.workspace(), response.settings);
      await runLoggedAction("Refresh workspace settings", () =>
        queryClient.invalidateQueries({ queryKey: queryKeys.settings.workspace() }),
      );
    },
  });
}
