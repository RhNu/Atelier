import { useQuery } from "@tanstack/react-query";

import { promptApi, queryKeys } from "@/platform/atelier";
import type { PromptLexiconListQueryDto } from "@/types";

export const LEXICON_BROWSE_LIMIT = 80;
export const LEXICON_SEARCH_LIMIT = 60;

export function usePromptLexiconCatalogQuery() {
  return useQuery({
    queryKey: queryKeys.prompt.lexiconCatalog(),
    queryFn: () => promptApi.lexiconCatalog(),
  });
}

export function usePromptLexiconBrowseQuery(request: PromptLexiconListQueryDto, enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.prompt.lexiconList(request),
    queryFn: () => promptApi.lexiconList(request),
    enabled,
  });
}

export function usePromptLexiconSearchQuery(query: string, enabled: boolean) {
  const request = { query, limit: LEXICON_SEARCH_LIMIT };
  return useQuery({
    queryKey: queryKeys.prompt.lexiconSearch(request),
    queryFn: () => promptApi.lexiconSearch(request),
    enabled: enabled && query.length > 0,
  });
}
