import { useQuery } from "@tanstack/react-query";

import { promptApi, queryKeys } from "@/platform/atelier";
import type { ImageModelDto } from "@/types";

export function usePromptChunkPickerQuery(model: ImageModelDto | null, enabled: boolean) {
  const request = { offset: 0, limit: 200, model } as const;
  return useQuery({
    queryKey: queryKeys.prompt.chunks(request),
    queryFn: () => promptApi.listChunks(request),
    enabled,
  });
}
