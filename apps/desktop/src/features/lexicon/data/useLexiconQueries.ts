import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { generationApi, lexiconApi, queryKeys } from "@/platform/atelier";
import type { AppendLexiconEntitiesRequestDto, LexiconSearchRequestDto } from "@/types";

const SEMANTIC_SEARCH_TIMEOUT_MS = 20_000;

export class SemanticSearchTimeoutError extends Error {
  constructor() {
    super("semantic search timed out");
    this.name = "SemanticSearchTimeoutError";
  }
}

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
    queryFn: () =>
      request.mode === "semantic"
        ? withTimeout(lexiconApi.search(request), SEMANTIC_SEARCH_TIMEOUT_MS)
        : lexiconApi.search(request),
    enabled,
    placeholderData: request.mode === "lexical" ? (previous) => previous : undefined,
    retry: request.mode === "semantic" ? false : undefined,
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

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => reject(new SemanticSearchTimeoutError()), timeoutMs);
    void promise
      .then((value) => {
        window.clearTimeout(timeout);
        resolve(value);
      })
      .catch((error: unknown) => {
        window.clearTimeout(timeout);
        reject(error);
      });
  });
}
