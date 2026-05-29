import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  generationApi,
  promptApi,
  queryKeys,
  resourceApi,
  settingsApi,
} from "../../../platform/atelier";
import type {
  CompilePromptRequestDto,
  ResourceRefDto,
  SubmitGenerationRequestDto,
} from "../../../types";

export function useGenerationSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}

export function useSubmitGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: SubmitGenerationRequestDto) => generationApi.submit(request),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function usePauseGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.pause(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useResumeGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.resume(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useStopGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.stop(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useCompilePromptMutation() {
  return useMutation({
    mutationFn: (request: CompilePromptRequestDto) => promptApi.compilePreview(request),
  });
}

export function useResourceImageQuery(resource: ResourceRefDto | null) {
  return useQuery({
    queryKey: resource ? queryKeys.resource.image(resource) : ["resource", "image", null],
    queryFn: () => {
      if (!resource) {
        throw new Error("resource is required");
      }
      return resourceApi.image({ resource });
    },
    enabled: Boolean(resource),
  });
}

async function invalidateGenerationWorkbench(queryClient: ReturnType<typeof useQueryClient>) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: queryKeys.generation.root() }),
    queryClient.invalidateQueries({ queryKey: queryKeys.history.root() }),
    queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
    queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
  ]);
}
