import { useQuery } from "@tanstack/react-query";

import { promptApi, queryKeys } from "../../../platform/atelier";
import type { PromptChunkDto, PromptLexiconEntryDto } from "../../../types";

type PromptCompletionQueryContext = {
  mode: "tag" | "chunk";
  query: string;
  manual: boolean;
} | null;

type PromptCompletionQueries = {
  chunks: PromptChunkDto[];
  tags: PromptLexiconEntryDto[];
};

const TAG_LIMIT = 12;
const CHUNK_LIMIT = 200;
const promptCompletionChunkQuery = {
  offset: 0,
  limit: CHUNK_LIMIT,
};

export function usePromptCompletionQueries({
  context,
  debouncedQuery,
}: {
  context: PromptCompletionQueryContext;
  debouncedQuery: string;
}): PromptCompletionQueries {
  const manualEmptyPicker = Boolean(context?.manual && context.query.length === 0);
  const canSearchTags =
    context?.mode === "tag" && debouncedQuery.trim().length > 0 && !manualEmptyPicker;
  const tagQueryIsCurrent = debouncedQuery.trim() === (context?.query.trim() ?? "");
  const chunksQuery = useQuery({
    queryKey: queryKeys.prompt.chunks(promptCompletionChunkQuery),
    queryFn: () => promptApi.listChunks(promptCompletionChunkQuery),
    staleTime: 30_000,
  });
  const lexiconQuery = useQuery({
    queryKey: queryKeys.prompt.lexiconSearch({ query: debouncedQuery, limit: TAG_LIMIT }),
    queryFn: () => promptApi.lexiconSearch({ query: debouncedQuery, limit: TAG_LIMIT }),
    enabled: canSearchTags,
    staleTime: 30_000,
  });

  return {
    chunks: chunksQuery.data?.items ?? [],
    tags: canSearchTags && tagQueryIsCurrent ? (lexiconQuery.data?.items ?? []) : [],
  };
}
