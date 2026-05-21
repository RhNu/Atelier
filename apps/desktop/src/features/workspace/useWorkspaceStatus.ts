import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  AtelierCommandError,
  clearWorkspaceScopedQueryCache,
  desktopApi,
  queryKeys,
  workspaceApi,
} from "../../platform/atelier";
import type { WorkspaceStatusDto } from "../../types";

export type WorkspaceStatusView = {
  workspaceStatus: WorkspaceStatusDto | null;
  workspacePending: boolean;
  workspaceErrorCode: string | undefined;
  workspaceErrorMessage: string | undefined;
  openWorkspace: () => void;
  closeWorkspace: () => void;
  openingWorkspace: boolean;
  closingWorkspace: boolean;
};

function isWorkspaceNotOpen(error: unknown): boolean {
  return error instanceof AtelierCommandError && error.code === "workspace_not_open";
}

function getCommandError(error: unknown): AtelierCommandError | null {
  return error instanceof AtelierCommandError ? error : null;
}

export function useWorkspaceStatus(): WorkspaceStatusView {
  const queryClient = useQueryClient();

  const statusQuery = useQuery({
    queryKey: queryKeys.workspace.status(),
    queryFn: async () => {
      try {
        return await workspaceApi.status();
      } catch (error) {
        if (isWorkspaceNotOpen(error)) {
          return null;
        }

        throw error;
      }
    },
    retry: (failureCount, error) => !isWorkspaceNotOpen(error) && failureCount < 1,
  });

  const openMutation = useMutation({
    mutationFn: async () => {
      const root = await desktopApi.pickWorkspaceDirectory();

      if (!root) {
        return null;
      }

      return workspaceApi.open({ root });
    },
    onSuccess: async (status) => {
      if (!status) {
        return;
      }

      await clearWorkspaceScopedQueryCache(queryClient);
      queryClient.setQueryData(queryKeys.workspace.status(), status);
      queryClient.invalidateQueries({ queryKey: queryKeys.workspace.root() });
    },
  });

  const closeMutation = useMutation({
    mutationFn: () => workspaceApi.close(),
    onSuccess: async () => {
      await clearWorkspaceScopedQueryCache(queryClient);
      queryClient.setQueryData(queryKeys.workspace.status(), null);
      queryClient.invalidateQueries({ queryKey: queryKeys.workspace.root() });
    },
  });

  const error = getCommandError(statusQuery.error ?? openMutation.error ?? closeMutation.error);

  return {
    workspaceStatus: statusQuery.data ?? null,
    workspacePending: statusQuery.isPending || openMutation.isPending || closeMutation.isPending,
    workspaceErrorCode: error?.code,
    workspaceErrorMessage: error?.message,
    openWorkspace: () => openMutation.mutate(),
    closeWorkspace: () => closeMutation.mutate(),
    openingWorkspace: openMutation.isPending,
    closingWorkspace: closeMutation.isPending,
  };
}
