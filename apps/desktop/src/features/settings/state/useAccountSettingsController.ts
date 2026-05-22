import { useCallback, useState } from "react";

import type { ApiKeyRecordDto, SubscriptionSummaryDto } from "../../../types";
import type { ApiKeyEditState, ApiKeyProbeState } from "../components/ApiKeyRow";
import {
  useApiKeysQuery,
  useCreateApiKeyMutation,
  useDeleteApiKeyMutation,
  useProbeActiveApiKeyMutation,
  useProbeApiKeyMutation,
  useSetActiveApiKeyMutation,
  useUpdateApiKeyMutation,
} from "../data/useAccountSettingsQueries";
import { createApiKeyId, formatError } from "../settings-utils";

type ProbeState = Record<string, ApiKeyProbeState>;

const emptyApiKeys: ApiKeyRecordDto[] = [];

function withoutProbeState(current: ProbeState, id: string): ProbeState {
  const next = { ...current };
  delete next[id];
  return next;
}

function toActiveProbeState(probe: {
  isPending: boolean;
  isError: boolean;
  error: unknown;
  data: SubscriptionSummaryDto | undefined;
}) {
  return {
    pending: probe.isPending,
    error: probe.isError ? formatError(probe.error) : null,
    summary: probe.data ?? null,
  };
}

export type AccountSettingsController = {
  keys: ApiKeyRecordDto[];
  keysPending: boolean;
  keysError: string | null;
  busy: boolean;
  createDisabled: boolean;
  displayName: string;
  secret: string;
  commandError: string | null;
  editing: ApiKeyEditState | null;
  probeState: ProbeState;
  activeProbe: {
    pending: boolean;
    error: string | null;
    summary: SubscriptionSummaryDto | null;
  };
  setDisplayName: (value: string) => void;
  setSecret: (value: string) => void;
  setEditing: (editing: ApiKeyEditState) => void;
  cancelEdit: () => void;
  createKey: () => void;
  probeKey: (key: ApiKeyRecordDto) => void;
  saveEdit: () => void;
  setActive: (id: string) => void;
  deleteKey: (id: string) => void;
  refreshActive: () => void;
};

export function useAccountSettingsController(): AccountSettingsController {
  const apiKeysQuery = useApiKeysQuery();
  const activeSubscriptionMutation = useProbeActiveApiKeyMutation();
  const createApiKeyMutation = useCreateApiKeyMutation();
  const updateApiKeyMutation = useUpdateApiKeyMutation();
  const deleteApiKeyMutation = useDeleteApiKeyMutation();
  const setActiveApiKeyMutation = useSetActiveApiKeyMutation();
  const probeApiKeyMutation = useProbeApiKeyMutation();
  const [displayName, setDisplayName] = useState("");
  const [secret, setSecret] = useState("");
  const [probeState, setProbeState] = useState<ProbeState>({});
  const [editing, setEditing] = useState<ApiKeyEditState | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const keys = apiKeysQuery.data ?? emptyApiKeys;
  const busy =
    createApiKeyMutation.isPending ||
    updateApiKeyMutation.isPending ||
    deleteApiKeyMutation.isPending ||
    setActiveApiKeyMutation.isPending ||
    probeApiKeyMutation.isPending ||
    activeSubscriptionMutation.isPending;

  const handleMutationError = useCallback((error: unknown) => {
    setCommandError(formatError(error));
  }, []);
  const clearActiveProbe = useCallback(() => {
    activeSubscriptionMutation.reset();
  }, [activeSubscriptionMutation]);
  const markKeyChanged = useCallback(
    (id: string) => {
      setCommandError(null);
      clearActiveProbe();
      setProbeState((current) => withoutProbeState(current, id));
    },
    [clearActiveProbe],
  );

  const createKey = useCallback(() => {
    const nextDisplayName = displayName.trim();
    const nextSecret = secret.trim();
    if (!nextDisplayName || !nextSecret) {
      return;
    }

    setCommandError(null);
    createApiKeyMutation.mutate(
      { id: createApiKeyId(), display_name: nextDisplayName, secret: nextSecret },
      {
        onSuccess: () => {
          setDisplayName("");
          setSecret("");
          clearActiveProbe();
        },
        onError: handleMutationError,
      },
    );
  }, [clearActiveProbe, createApiKeyMutation, displayName, handleMutationError, secret]);

  const probeKey = useCallback(
    (key: ApiKeyRecordDto) => {
      probeApiKeyMutation.mutate(
        { id: key.id },
        {
          onSuccess: (subscription) => {
            setProbeState((current) => ({
              ...current,
              [key.id]: { subscription, error: null },
            }));
          },
          onError: (error) => {
            setProbeState((current) => ({
              ...current,
              [key.id]: { subscription: null, error: formatError(error) },
            }));
          },
        },
      );
    },
    [probeApiKeyMutation],
  );

  const saveEdit = useCallback(() => {
    if (!editing) {
      return;
    }

    const nextDisplayName = editing.displayName.trim();
    const nextSecret = editing.secret.trim();
    if (!nextDisplayName && !nextSecret) {
      return;
    }

    setCommandError(null);
    updateApiKeyMutation.mutate(
      { id: editing.id, display_name: nextDisplayName || null, secret: nextSecret || null },
      {
        onSuccess: () => {
          markKeyChanged(editing.id);
          setEditing(null);
        },
        onError: handleMutationError,
      },
    );
  }, [editing, handleMutationError, markKeyChanged, updateApiKeyMutation]);

  const setActive = useCallback(
    (id: string) => {
      setCommandError(null);
      setActiveApiKeyMutation.mutate(
        { id },
        { onSuccess: () => markKeyChanged(id), onError: handleMutationError },
      );
    },
    [handleMutationError, markKeyChanged, setActiveApiKeyMutation],
  );

  const deleteKey = useCallback(
    (id: string) => {
      setCommandError(null);
      deleteApiKeyMutation.mutate(
        { id },
        { onSuccess: () => markKeyChanged(id), onError: handleMutationError },
      );
    },
    [deleteApiKeyMutation, handleMutationError, markKeyChanged],
  );

  const refreshActive = useCallback(() => {
    setCommandError(null);
    activeSubscriptionMutation.mutate(undefined, { onError: handleMutationError });
  }, [activeSubscriptionMutation, handleMutationError]);

  return {
    keys,
    keysPending: apiKeysQuery.isPending,
    keysError: apiKeysQuery.isError ? formatError(apiKeysQuery.error) : null,
    busy,
    createDisabled: busy || displayName.trim() === "" || secret.trim() === "",
    displayName,
    secret,
    commandError,
    editing,
    probeState,
    activeProbe: toActiveProbeState(activeSubscriptionMutation),
    setDisplayName,
    setSecret,
    setEditing,
    cancelEdit: () => setEditing(null),
    createKey,
    probeKey,
    saveEdit,
    setActive,
    deleteKey,
    refreshActive,
  };
}
