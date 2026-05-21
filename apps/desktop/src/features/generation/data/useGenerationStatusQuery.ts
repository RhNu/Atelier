import { useQuery } from "@tanstack/react-query";

import { generationApi, historyApi, queryKeys } from "../../../platform/atelier";
import type { RunHistoryQueryDto } from "../../../types";

const latestHistoryQuery: RunHistoryQueryDto = {
  offset: 0,
  limit: 8,
  kind: null,
  status: null,
};

export function useGenerationStatusQuery() {
  return useQuery({
    queryKey: queryKeys.generation.status(null),
    queryFn: () => generationApi.status({ job_id: null }),
  });
}

export function useLatestRunHistoryQuery() {
  return useQuery({
    queryKey: queryKeys.history.list(latestHistoryQuery),
    queryFn: () => historyApi.list(latestHistoryQuery),
  });
}
