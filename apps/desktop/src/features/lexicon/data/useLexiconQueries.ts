import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { generationApi, lexiconApi, queryKeys } from "@/platform/atelier";
import type { AppendLexiconEntitiesRequestDto, LexiconSearchRequestDto } from "@/types";

export function useLexiconBootstrapQuery() {
  return useQuery({
    queryKey: queryKeys.lexicon.bootstrap(),
    queryFn: () => lexiconApi.bootstrap(),
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useLexiconSearchQuery(request: LexiconSearchRequestDto) {
  const enabled = request.mode === "lexical" || request.query.trim().length > 0;
  return useQuery({
    queryKey: queryKeys.lexicon.search(request),
    queryFn: () => lexiconApi.search(request),
    enabled,
    placeholderData: (previous) => previous,
  });
}

export function useLexiconEntityQuery(entityId: number | null) {
  return useQuery({
    queryKey: queryKeys.lexicon.entity(entityId),
    queryFn: () => {
      if (entityId === null) throw new Error("lexicon entity ID is required");
      return lexiconApi.entity({ entity_id: entityId });
    },
    enabled: entityId !== null,
  });
}

export function useAppendLexiconEntitiesMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: AppendLexiconEntitiesRequestDto) =>
      generationApi.appendLexiconEntities(request),
    onSuccess: (draft) => {
      queryClient.setQueryData(queryKeys.generation.draft(), draft);
    },
  });
}
