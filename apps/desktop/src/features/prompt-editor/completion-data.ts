import type { QueryClient } from "@tanstack/react-query";

import { promptApi, queryKeys } from "@/platform/atelier";
import type { PromptChunkDto, PromptLexiconEntryDto } from "@/types";

const CHUNK_QUERY = { offset: 0, limit: 200 } as const;
const LEXICON_LIMIT = 20;
const COMPLETION_STALE_TIME_MS = 30_000;

export async function fetchPromptCompletionChunks(
  queryClient: QueryClient,
): Promise<PromptChunkDto[]> {
  const page = await queryClient.fetchQuery({
    queryKey: queryKeys.prompt.chunks(CHUNK_QUERY),
    queryFn: () => promptApi.listChunks(CHUNK_QUERY),
    staleTime: COMPLETION_STALE_TIME_MS,
  });
  return page.items;
}

export async function fetchPromptCompletionTags(
  queryClient: QueryClient,
  query: string,
): Promise<PromptLexiconEntryDto[]> {
  const request = { query, limit: LEXICON_LIMIT };
  const page = await queryClient.fetchQuery({
    queryKey: queryKeys.prompt.lexiconSearch(request),
    queryFn: () => promptApi.lexiconSearch(request),
    staleTime: COMPLETION_STALE_TIME_MS,
  });
  return page.items;
}
