import { useCallback } from "react";

import { AppButton, AppPanel, EmptyState } from "@/components/ui";
import type { ApiKeyRecordDto } from "@/types";

import {
  isMissingActiveKey,
  useAccountSettingsController,
} from "../state/useAccountSettingsController";
import { ActiveSubscriptionPanel } from "./ActiveSubscriptionPanel";
import { ApiKeyRow, type ApiKeyEditState } from "./ApiKeyRow";
import { LoadingPanel, SectionHeader, TextField } from "./SettingsControls";

export function AccountSettingsSection() {
  const { t } = useTranslation("settings");
  const account = useAccountSettingsController();
  const retryActiveSummary = useCallback(() => {
    void account.activeSummary.refetch();
  }, [account.activeSummary]);

  return (
    <div className="h-full min-h-0">
      <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
        <SectionHeader
          kicker={t("account")}
          title={t("accountTitle")}
          description={t("accountDescriptionLong")}
        />
        <div className="min-h-0 flex-1 overflow-auto p-3">
          <ActiveSubscriptionPanel
            pending={account.activeSummary.isPending}
            refreshing={account.activeSummary.isFetching && !account.activeSummary.isPending}
            missingActiveKey={isMissingActiveKey(account.activeSummary.error)}
            error={account.activeSummary.isError ? account.activeSummary.error : null}
            summary={account.activeSummary.data ?? null}
            onRetry={retryActiveSummary}
          />
          <CreateKeyForm
            displayName={account.displayName}
            secret={account.secret}
            disabled={account.createDisabled}
            onDisplayNameChange={account.setDisplayName}
            onSecretChange={account.setSecret}
            onCreate={account.createKey}
          />
          <ApiKeyList
            keys={account.keys}
            pending={account.keysPending}
            error={account.keysError}
            editing={account.editing}
            busy={account.busy}
            onEdit={account.setEditing}
            onCancelEdit={account.cancelEdit}
            onEditChange={account.setEditing}
            onSaveEdit={account.saveEdit}
            onSetActive={account.setActive}
            onDelete={account.deleteKey}
          />
        </div>
      </AppPanel>
    </div>
  );
}

function CreateKeyForm({
  displayName,
  secret,
  disabled,
  onDisplayNameChange,
  onSecretChange,
  onCreate,
}: {
  displayName: string;
  secret: string;
  disabled: boolean;
  onDisplayNameChange: (value: string) => void;
  onSecretChange: (value: string) => void;
  onCreate: () => void;
}) {
  const { t } = useTranslation("settings");
  return (
    <div className="grid gap-3 border border-app-border/70 bg-app-surface/60 p-3 md:grid-cols-[1fr_1.4fr_auto]">
      <TextField
        label={t("apiKeyDisplayName")}
        value={displayName}
        onChange={onDisplayNameChange}
        placeholder={t("mainKeyPlaceholder")}
      />
      <TextField
        label={t("apiKeySecret")}
        value={secret}
        onChange={onSecretChange}
        placeholder={t("pasteKeyPlaceholder")}
        type="password"
        autoComplete="new-password"
      />
      <div className="flex items-end">
        <AppButton className="w-full" disabled={disabled} onClick={onCreate}>
          {t("addApiKey")}
        </AppButton>
      </div>
    </div>
  );
}

function ApiKeyList({
  keys,
  pending,
  error,
  editing,
  busy,
  onEdit,
  onCancelEdit,
  onEditChange,
  onSaveEdit,
  onSetActive,
  onDelete,
}: {
  keys: ApiKeyRecordDto[];
  pending: boolean;
  error: string | null;
  editing: ApiKeyEditState | null;
  busy: boolean;
  onEdit: (editing: ApiKeyEditState) => void;
  onCancelEdit: () => void;
  onEditChange: (editing: ApiKeyEditState) => void;
  onSaveEdit: () => void;
  onSetActive: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation("settings");
  if (pending) {
    return <LoadingPanel label={t("loadingApiKeys")} />;
  }

  if (error) {
    return <EmptyState title={t("apiKeysUnavailable")} description={error} />;
  }

  if (keys.length === 0) {
    return <EmptyState title={t("noApiKeys")} />;
  }

  return (
    <div className="mt-3 border border-app-border bg-app-surface/45">
      {keys.map((key) => (
        <ApiKeyRow
          key={key.id}
          item={key}
          editing={editing}
          busy={busy}
          onEdit={onEdit}
          onCancelEdit={onCancelEdit}
          onEditChange={onEditChange}
          onSaveEdit={onSaveEdit}
          onSetActive={onSetActive}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
}
import { useTranslation } from "react-i18next";
