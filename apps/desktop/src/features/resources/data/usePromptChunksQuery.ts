import { useQuery } from "@tanstack/react-query";

import { promptApi, queryKeys } from "../../../platform/atelier";

const promptChunkQuery = {
  offset: 0,
  limit: 24,
};

export function usePromptChunksQuery() {
  return useQuery({
    queryKey: queryKeys.prompt.chunks(),
    queryFn: () => promptApi.listChunks(promptChunkQuery),
  });
}
