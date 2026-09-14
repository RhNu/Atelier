import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";

import { agentApi, queryKeys } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type { AgentEventDto, AgentSessionDto } from "@/types";

import {
  useAgentEventsQuery,
  useAgentRegistryQuery,
  useAgentSessionMutations,
  useAgentSessionsQuery,
  useUpdateAgentWorkspaceSettingsMutation,
  useAgentWorkspaceSettingsQuery,
} from "../data/useAgentQueries";
import { useAgentDrawerStore } from "../state/agent-drawer-store";
import { AgentDrawerView } from "./AgentDrawerView";

type AgentDrawerProps = {
  route: string;
  workspaceId: string;
  onOpenSettings: () => void;
};

const EMPTY_EVENTS: AgentEventDto[] = [];

// eslint-disable-next-line max-lines-per-function
export function AgentDrawer({ route, workspaceId, onOpenSettings }: AgentDrawerProps) {
  const { t } = useTranslation("agent");
  const queryClient = useQueryClient();
  const pushToast = useToastStore((state) => state.push);
  const drawer = useAgentDrawerStore();
  const registry = useAgentRegistryQuery();
  const settings = useAgentWorkspaceSettingsQuery(true);
  const sessions = useAgentSessionsQuery(true);
  const sessionMutations = useAgentSessionMutations();
  const settingsMutation = useUpdateAgentWorkspaceSettingsMutation();
  const events = useAgentEventsQuery(drawer.activeSessionId, true);
  const [message, setMessage] = useState("");
  const running = drawer.runningSessionId !== null;
  useAgentDrawerLifecycle({
    workspaceId,
    activeSessionId: drawer.activeSessionId,
    firstSessionId: sessions.data?.[0]?.id ?? null,
    resetWorkspace: drawer.resetWorkspace,
    selectSession: drawer.selectSession,
    toggle: drawer.toggle,
  });

  const sessionOptions = useAgentSessionOptions(sessions.data);
  const activeSession = sessions.data?.find((session) => session.id === drawer.activeSessionId);
  const defaultModelId = resolveDefaultModelId(
    settings.data?.default_model_id,
    registry.data?.models ?? [],
  );

  const createSession = useCallback(() => {
    if (!defaultModelId) return;
    sessionMutations.create.mutate(
      { title: t("newSessionTitle"), model_id: defaultModelId },
      {
        onSuccess: (session) => drawer.selectSession(session.id),
        onError: (error) => notifyError(pushToast, t("sessionCreateFailed"), error),
      },
    );
  }, [defaultModelId, drawer, pushToast, sessionMutations.create, t]);

  const removeSession = useCallback(() => {
    if (!drawer.activeSessionId) return;
    sessionMutations.remove.mutate(
      { session_id: drawer.activeSessionId },
      {
        onSuccess: () => drawer.selectSession(null),
        onError: (error) => notifyError(pushToast, t("sessionDeleteFailed"), error),
      },
    );
  }, [drawer, pushToast, sessionMutations.remove, t]);

  const renameSession = useCallback(
    (title: string) => {
      if (!drawer.activeSessionId) return;
      sessionMutations.rename.mutate(
        { session_id: drawer.activeSessionId, title },
        {
          onError: (error) => notifyError(pushToast, t("sessionRenameFailed"), error),
        },
      );
    },
    [drawer.activeSessionId, pushToast, sessionMutations.rename, t],
  );

  const changePermissionMode = useCallback(
    (permissionMode: NonNullable<typeof settings.data>["permission_mode"]) => {
      if (!settings.data) return;
      settingsMutation.mutate(
        { settings: { ...settings.data, permission_mode: permissionMode } },
        {
          onError: (error) => notifyError(pushToast, t("personaSaveFailed"), error),
        },
      );
    },
    [pushToast, settings.data, settingsMutation, t],
  );

  const send = useCallback(async () => {
    const content = message.trim();
    const sessionId = drawer.activeSessionId;
    if (!content || !sessionId || running) return;
    setMessage("");
    drawer.beginTurn(sessionId, content);
    try {
      await agentApi.runTurn(
        {
          session_id: sessionId,
          message: content,
          context: { route, selected_resource_ids: [] },
        },
        drawer.pushEvent,
      );
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.agent.events(sessionId) }),
        queryClient.invalidateQueries({ queryKey: queryKeys.agent.sessions() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.generation.draft() }),
        queryClient.invalidateQueries({ queryKey: queryKeys.generation.root() }),
      ]);
      drawer.finishTurn();
    } catch (error) {
      await queryClient.invalidateQueries({ queryKey: queryKeys.agent.events(sessionId) });
      drawer.finishTurn(formatError(error));
    }
  }, [drawer, message, queryClient, route, running]);

  const stop = useCallback(() => {
    if (!drawer.runningSessionId) return;
    void agentApi
      .cancelTurn({ session_id: drawer.runningSessionId })
      .catch((error) => notifyError(pushToast, t("stopFailed"), error));
  }, [drawer.runningSessionId, pushToast, t]);

  const decideApproval = useCallback(
    (approvalId: string, approved: boolean) => {
      void agentApi
        .decideApproval({ approval_id: approvalId, approved })
        .catch((error) => notifyError(pushToast, t("approvalFailed"), error));
    },
    [pushToast, t],
  );

  const undo = useCallback(
    (actionId: string) => {
      void agentApi
        .undoAction({ action_id: actionId })
        .then((draft) => {
          queryClient.setQueryData(queryKeys.generation.draft(), draft);
          pushToast({ level: "success", message: t("undoSucceeded") });
        })
        .catch((error) => notifyError(pushToast, t("undoFailed"), error));
    },
    [pushToast, queryClient, t],
  );

  const resize = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      const startX = event.clientX;
      const startWidth = drawer.width;
      const move = (moveEvent: globalThis.PointerEvent) =>
        drawer.setWidth(startWidth + startX - moveEvent.clientX);
      const finish = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", finish);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", finish, { once: true });
    },
    [drawer],
  );

  const close = useCallback(() => drawer.setOpen(false), [drawer]);
  const sendFromView = useCallback(() => void send(), [send]);
  return (
    <AgentDrawerView
      open={drawer.open}
      width={drawer.width}
      activeSessionId={drawer.activeSessionId}
      activeSessionTitle={activeSession?.title ?? ""}
      contextWindow={activeSession?.model.context_window ?? null}
      contextInputTokens={drawer.contextInputTokens}
      permissionMode={settings.data?.permission_mode ?? "standard"}
      updatingPermission={settingsMutation.isPending || !settings.data}
      defaultModelId={defaultModelId}
      running={running}
      creating={sessionMutations.create.isPending}
      deleting={sessionMutations.remove.isPending}
      renaming={sessionMutations.rename.isPending}
      sessionOptions={sessionOptions}
      events={events.data ?? EMPTY_EVENTS}
      liveEvents={drawer.liveEvents}
      pendingUserMessage={drawer.pendingUserMessage}
      error={drawer.error}
      message={message}
      onMessageChange={setMessage}
      onClose={close}
      onResize={resize}
      onSelectSession={drawer.selectSession}
      onCreateSession={createSession}
      onDeleteSession={removeSession}
      onRenameSession={renameSession}
      onPermissionModeChange={changePermissionMode}
      onOpenSettings={onOpenSettings}
      onApproval={decideApproval}
      onUndo={undo}
      onSend={sendFromView}
      onStop={stop}
    />
  );
}

function useAgentSessionOptions(sessions: AgentSessionDto[] | undefined) {
  return useMemo(
    () =>
      sessions?.map((session) => ({
        value: session.id,
        label: `${session.title} · ${session.model.display_name}`,
      })) ?? [],
    [sessions],
  );
}

function useAgentDrawerLifecycle({
  workspaceId,
  activeSessionId,
  firstSessionId,
  resetWorkspace,
  selectSession,
  toggle,
}: {
  workspaceId: string;
  activeSessionId: string | null;
  firstSessionId: string | null;
  resetWorkspace: () => void;
  selectSession: (sessionId: string | null) => void;
  toggle: () => void;
}) {
  const previousWorkspaceRef = useRef(workspaceId);
  useEffect(() => {
    if (previousWorkspaceRef.current !== workspaceId) {
      previousWorkspaceRef.current = workspaceId;
      resetWorkspace();
    }
  }, [resetWorkspace, workspaceId]);
  useEffect(() => {
    if (!activeSessionId && firstSessionId) selectSession(firstSessionId);
  }, [activeSessionId, firstSessionId, selectSession]);
  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.shiftKey && event.key.toLocaleLowerCase() === "a") {
        event.preventDefault();
        toggle();
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [toggle]);
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function resolveDefaultModelId(
  preferredModelId: string | null | undefined,
  models: ReadonlyArray<{ id: string }>,
): string | null {
  return models.find((model) => model.id === preferredModelId)?.id ?? models.at(0)?.id ?? null;
}

function notifyError(
  push: ReturnType<typeof useToastStore.getState>["push"],
  title: string,
  error: unknown,
) {
  push({ level: "error", title, message: formatError(error) });
}
