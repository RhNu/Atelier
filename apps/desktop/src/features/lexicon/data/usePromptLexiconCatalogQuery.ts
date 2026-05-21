import { useQuery } from "@tanstack/react-query";

import { promptApi, queryKeys } from "../../../platform/atelier";

export function usePromptLexiconCatalogQuery() {
  return useQuery({
    queryKey: queryKeys.prompt.lexiconCatalog(),
    queryFn: () => promptApi.lexiconCatalog(),
  });
}
