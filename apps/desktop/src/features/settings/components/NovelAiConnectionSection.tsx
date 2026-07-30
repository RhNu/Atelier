import { Plus } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { reportBackgroundPromise } from "@/app/logger";
import { AppButton, AppIconButton, AppModal, EmptyState } from "@/components/ui";
import type { ApiKeyRecordDto } from "@/types";

import {
  isMissingActiveKey,
  useAccountSettingsController,
} from "../state/useAccountSettingsController";
import { ActiveSubscriptionPanel } from "./ActiveSubscriptionPanel";
import { ApiKeyRow, type ApiKeyEditState } from "./ApiKeyRow";
import { LoadingPanel, TextField } from "./SettingsControls";

export function NovelAiConnectionSection() {
  const { t } = useTranslation("settings");
  const account = useAccountSettingsController();
  const refetchActiveSummary = account.activeSummary.refetch;
  const retryActiveSummary = useCallback(() => {
    reportBackgroundPromise(refetchActiveSummary(), "Retry active account summary");
  }, [refetchActiveSummary]);

  return (
    <>
      <section
        className="w-full border border-app-border bg-app-surface/45"
        aria-labelledby="novelai-connection-title"
      >
        <header className="flex flex-wrap items-center justify-between gap-3 p-4">
          <h2 id="novelai-connection-title" className="text-sm font-semibold text-white">
            {t("novelAiServiceName")}
          </h2>
          <AppIconButton icon={Plus} label={t("addApiKey")} onClick={account.openCreate} />
        </header>
        <ActiveSubscriptionPanel
          pending={account.activeSummary.isPending}
          refreshing={account.activeSummary.isFetching && !account.activeSummary.isPending}
          missingActiveKey={isMissingActiveKey(account.activeSummary.error)}
          error={account.activeSummary.isError ? account.activeSummary.error : null}
          summary={account.activeSummary.data ?? null}
          onRetry={retryActiveSummary}
        />
        <ApiKeyList
          keys={account.keys}
          pending={account.keysPending}
          error={account.keysError}
          busy={account.busy}
          onEdit={account.setEditing}
          onSetActive={account.setActive}
          onDelete={account.deleteKey}
        />
      </section>
      <CreateKeyModal
        open={account.createOpen}
        displayName={account.displayName}
        secret={account.secret}
        disabled={account.createDisabled}
        onDisplayNameChange={account.setDisplayName}
        onSecretChange={account.setSecret}
        onCreate={account.createKey}
        onClose={account.cancelCreate}
      />
      <EditKeyModal
        editing={account.editing}
        busy={account.busy}
        onEditChange={account.setEditing}
        onSave={account.saveEdit}
        onClose={account.cancelEdit}
      />
    </>
  );
}

function CreateKeyModal({
  open,
  displayName,
  secret,
  disabled,
  onDisplayNameChange,
  onSecretChange,
  onCreate,
  onClose,
}: {
  open: boolean;
  displayName: string;
  secret: string;
  disabled: boolean;
  onDisplayNameChange: (value: string) => void;
  onSecretChange: (value: string) => void;
  onCreate: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  return (
    <AppModal open={open} title={t("addApiKey")} onClose={onClose}>
      <div className="grid gap-4">
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
        <div className="flex justify-end gap-2 border-t border-app-border pt-3">
          <AppButton variant="ghost" onClick={onClose}>
            {translateCommon("cancel")}
          </AppButton>
          <AppButton disabled={disabled} onClick={onCreate}>
            {translateCommon("add")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}

function EditKeyModal({
  editing,
  busy,
  onEditChange,
  onSave,
  onClose,
}: {
  editing: ApiKeyEditState | null;
  busy: boolean;
  onEditChange: (editing: ApiKeyEditState) => void;
  onSave: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  const updateDisplayName = useCallback(
    (displayName: string) => {
      if (editing) onEditChange({ ...editing, displayName });
    },
    [editing, onEditChange],
  );
  const updateSecret = useCallback(
    (secret: string) => {
      if (editing) onEditChange({ ...editing, secret });
    },
    [editing, onEditChange],
  );
  const saveDisabled =
    busy || !editing || (editing.displayName.trim() === "" && editing.secret.trim() === "");

  return (
    <AppModal
      open={Boolean(editing)}
      title={t("editKey", { name: editing?.displayName ?? "" })}
      onClose={onClose}
    >
      {editing ? (
        <div className="grid gap-4">
          <TextField
            label={t("editApiKeyName")}
            value={editing.displayName}
            onChange={updateDisplayName}
            disabled={busy}
          />
          <TextField
            label={t("replaceApiKeySecret")}
            value={editing.secret}
            onChange={updateSecret}
            type="password"
            autoComplete="new-password"
            disabled={busy}
          />
          <div className="flex justify-end gap-2 border-t border-app-border pt-3">
            <AppButton variant="ghost" disabled={busy} onClick={onClose}>
              {translateCommon("cancel")}
            </AppButton>
            <AppButton disabled={saveDisabled} aria-label={t("saveApiKeyChanges")} onClick={onSave}>
              {translateCommon("save")}
            </AppButton>
          </div>
        </div>
      ) : null}
    </AppModal>
  );
}

function ApiKeyList({
  keys,
  pending,
  error,
  busy,
  onEdit,
  onSetActive,
  onDelete,
}: {
  keys: ApiKeyRecordDto[];
  pending: boolean;
  error: string | null;
  busy: boolean;
  onEdit: (editing: ApiKeyEditState) => void;
  onSetActive: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation("settings");
  if (pending) return <LoadingPanel label={t("loadingApiKeys")} />;
  if (error) return <EmptyState title={t("apiKeysUnavailable")} description={error} />;
  if (keys.length === 0) return <EmptyState title={t("noApiKeys")} />;

  return (
    <div className="max-h-72 overflow-y-auto">
      {keys.map((key) => (
        <ApiKeyRow
          key={key.id}
          item={key}
          busy={busy}
          onEdit={onEdit}
          onSetActive={onSetActive}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
}
