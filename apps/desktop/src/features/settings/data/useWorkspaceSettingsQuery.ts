import { useQuery } from "@tanstack/react-query";

import { queryKeys, settingsApi } from "../../../platform/atelier";

export function useWorkspaceSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}
