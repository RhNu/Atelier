import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { useActiveAccountSummaryQuery } from "@/features/account/data/useActiveAccountSummaryQuery";
import { useToastStore } from "@/stores/toast-store";
import type { ApiKeyRecordDto } from "@/types";

import type { ApiKeyEditState } from "../components/ApiKeyRow";
import {
  useApiKeysQuery,
  useCreateApiKeyMutation,
  useDeleteApiKeyMutation,
  useSetActiveApiKeyMutation,
  useUpdateApiKeyMutation,
} from "../data/useAccountSettingsQueries";
import { createApiKeyId, formatError } from "../settings-utils";

const emptyApiKeys: ApiKeyRecordDto[] = [];

export type AccountSettingsController = {
  keys: ApiKeyRecordDto[];
  keysPending: boolean;
  keysError: string | null;
  busy: boolean;
  createDisabled: boolean;
  displayName: string;
  secret: string;
  editing: ApiKeyEditState | null;
  activeSummary: ReturnType<typeof useActiveAccountSummaryQuery>;
  setDisplayName: (value: string) => void;
  setSecret: (value: string) => void;
  setEditing: (editing: ApiKeyEditState) => void;
  cancelEdit: () => void;
  createKey: () => void;
  saveEdit: () => void;
  setActive: (id: string) => void;
  deleteKey: (id: string) => void;
};

export function useAccountSettingsController(): AccountSettingsController {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const apiKeysQuery = useApiKeysQuery();
  const activeSummary = useActiveAccountSummaryQuery();
  const createApiKeyMutation = useCreateApiKeyMutation();
  const updateApiKeyMutation = useUpdateApiKeyMutation();
  const deleteApiKeyMutation = useDeleteApiKeyMutation();
  const setActiveApiKeyMutation = useSetActiveApiKeyMutation();
  const [displayName, setDisplayName] = useState("");
  const [secret, setSecret] = useState("");
  const [editing, setEditing] = useState<ApiKeyEditState | null>(null);
  const keys = apiKeysQuery.data ?? emptyApiKeys;
  const busy =
    createApiKeyMutation.isPending ||
    updateApiKeyMutation.isPending ||
    deleteApiKeyMutation.isPending ||
    setActiveApiKeyMutation.isPending;

  const handleMutationError = useCallback(
    (error: unknown) => {
      pushToast({ level: "error", title: t("accountActionFailed"), message: formatError(error) });
    },
    [pushToast, t],
  );

  const createKey = useCallback(() => {
    const nextDisplayName = displayName.trim();
    const nextSecret = secret.trim();
    if (!nextDisplayName || !nextSecret) {
      return;
    }

    createApiKeyMutation.mutate(
      { id: createApiKeyId(), display_name: nextDisplayName, secret: nextSecret },
      {
        onSuccess: () => {
          setDisplayName("");
          setSecret("");
          pushToast({ level: "success", message: t("apiKeyAdded") });
        },
        onError: handleMutationError,
      },
    );
  }, [createApiKeyMutation, displayName, handleMutationError, pushToast, secret, t]);

  const saveEdit = useCallback(() => {
    if (!editing) {
      return;
    }

    const nextDisplayName = editing.displayName.trim();
    const nextSecret = editing.secret.trim();
    if (!nextDisplayName && !nextSecret) {
      return;
    }

    updateApiKeyMutation.mutate(
      { id: editing.id, display_name: nextDisplayName || null, secret: nextSecret || null },
      {
        onSuccess: () => {
          setEditing(null);
          pushToast({ level: "success", message: t("apiKeyUpdated") });
        },
        onError: handleMutationError,
      },
    );
  }, [editing, handleMutationError, pushToast, t, updateApiKeyMutation]);

  const setActive = useCallback(
    (id: string) => {
      setActiveApiKeyMutation.mutate(
        { id },
        {
          onSuccess: () => pushToast({ level: "success", message: t("activeApiKeyChanged") }),
          onError: handleMutationError,
        },
      );
    },
    [handleMutationError, pushToast, setActiveApiKeyMutation, t],
  );

  const deleteKey = useCallback(
    (id: string) => {
      deleteApiKeyMutation.mutate(
        { id },
        {
          onSuccess: () => pushToast({ level: "success", message: t("apiKeyDeleted") }),
          onError: handleMutationError,
        },
      );
    },
    [deleteApiKeyMutation, handleMutationError, pushToast, t],
  );

  return {
    keys,
    keysPending: apiKeysQuery.isPending,
    keysError: apiKeysQuery.isError ? formatError(apiKeysQuery.error) : null,
    busy,
    createDisabled: busy || displayName.trim() === "" || secret.trim() === "",
    displayName,
    secret,
    editing,
    activeSummary,
    setDisplayName,
    setSecret,
    setEditing,
    cancelEdit: () => setEditing(null),
    createKey,
    saveEdit,
    setActive,
    deleteKey,
  };
}

export function isMissingActiveKey(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    error.code === "missing_active_key"
  );
}
