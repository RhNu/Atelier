import { useQuery } from "@tanstack/react-query";

import { globalSettingsApi, queryKeys } from "../../../platform/atelier";

export function useGenerationGlobalSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.app.globalSettings(),
    queryFn: () => globalSettingsApi.get(),
  });
}
