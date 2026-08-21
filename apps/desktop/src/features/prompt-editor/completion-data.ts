import type { QueryClient } from "@tanstack/react-query";

import { lexiconApi, promptApi, queryKeys } from "@/platform/atelier";
import type { ImageModelDto, LexiconSearchItemDto, PromptChunkDto } from "@/types";

const LEXICON_LIMIT = 20;
const COMPLETION_STALE_TIME_MS = 30_000;

export async function fetchPromptCompletionChunks(
  queryClient: QueryClient,
  model: ImageModelDto | null = null,
): Promise<PromptChunkDto[]> {
  const query = { offset: 0, limit: 200, model } as const;
  const page = await queryClient.fetchQuery({
    queryKey: queryKeys.prompt.chunks(query),
    queryFn: () => promptApi.listChunks(query),
    staleTime: COMPLETION_STALE_TIME_MS,
  });
  return page.items;
}

export async function fetchPromptCompletionTags(
  queryClient: QueryClient,
  query: string,
): Promise<LexiconSearchItemDto[]> {
  const request = { query, limit: LEXICON_LIMIT };
  return queryClient.fetchQuery({
    queryKey: queryKeys.lexicon.completion(query, LEXICON_LIMIT),
    queryFn: () => lexiconApi.complete(request),
    staleTime: COMPLETION_STALE_TIME_MS,
  });
}
