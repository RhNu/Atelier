import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { agentApi, queryKeys } from "@/platform/atelier";
import type {
  CreateAgentSessionRequestDto,
  DeleteAgentConnectionRequestDto,
  DeleteAgentModelRequestDto,
  DeleteAgentSessionRequestDto,
  RenameAgentSessionRequestDto,
  SaveAgentConnectionRequestDto,
  SaveAgentModelRequestDto,
  UpdateAgentWorkspaceSettingsRequestDto,
} from "@/types";

export function useAgentRegistryQuery() {
  return useQuery({ queryKey: queryKeys.agent.registry(), queryFn: agentApi.registry });
}

export function useAgentWorkspaceSettingsQuery(enabled = true) {
  return useQuery({
    queryKey: queryKeys.agent.settings(),
    queryFn: agentApi.workspaceSettings,
    enabled,
  });
}

export function useAgentSessionsQuery(enabled = true) {
  return useQuery({
    queryKey: queryKeys.agent.sessions(),
    queryFn: agentApi.listSessions,
    enabled,
  });
}

export function useAgentEventsQuery(sessionId: string | null, enabled = true) {
  return useQuery({
    queryKey: queryKeys.agent.events(sessionId),
    queryFn: () => agentApi.listEvents({ session_id: requireSessionId(sessionId) }),
    enabled: enabled && Boolean(sessionId),
  });
}

export function useAgentRegistryMutations() {
  const queryClient = useQueryClient();
  const commit = async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.agent.registry() });
  };
  return {
    saveConnection: useMutation({
      mutationFn: (request: SaveAgentConnectionRequestDto) => agentApi.saveConnection(request),
      onSuccess: commit,
    }),
    deleteConnection: useMutation({
      mutationFn: (request: DeleteAgentConnectionRequestDto) => agentApi.deleteConnection(request),
      onSuccess: commit,
    }),
    saveModel: useMutation({
      mutationFn: (request: SaveAgentModelRequestDto) => agentApi.saveModel(request),
      onSuccess: commit,
    }),
    deleteModel: useMutation({
      mutationFn: (request: DeleteAgentModelRequestDto) => agentApi.deleteModel(request),
      onSuccess: commit,
    }),
  };
}

export function useUpdateAgentWorkspaceSettingsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: UpdateAgentWorkspaceSettingsRequestDto) =>
      agentApi.updateWorkspaceSettings(request),
    onSuccess: (settings) => queryClient.setQueryData(queryKeys.agent.settings(), settings),
  });
}

export function useAgentSessionMutations() {
  const queryClient = useQueryClient();
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.agent.sessions() });
  };
  return {
    create: useMutation({
      mutationFn: (request: CreateAgentSessionRequestDto) => agentApi.createSession(request),
      onSuccess: refresh,
    }),
    rename: useMutation({
      mutationFn: (request: RenameAgentSessionRequestDto) => agentApi.renameSession(request),
      onSuccess: refresh,
    }),
    remove: useMutation({
      mutationFn: (request: DeleteAgentSessionRequestDto) => agentApi.deleteSession(request),
      onSuccess: refresh,
    }),
  };
}

function requireSessionId(sessionId: string | null): string {
  if (!sessionId) throw new Error("Agent session is required");
  return sessionId;
}
