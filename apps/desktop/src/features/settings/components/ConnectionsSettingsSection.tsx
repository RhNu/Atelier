import { ExternalLink, PencilLine, Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppPanel } from "@/components/ui";
import {
  useDanbooruAccountMutations,
  useDanbooruAccountQuery,
} from "@/features/inspiration/data/useDanbooruQueries";
import { desktopApi } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";

import { LoadingPanel, SectionHeader, TextField } from "./SettingsControls";

export function ConnectionsSettingsSection() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const account = useDanbooruAccountQuery();
  const mutations = useDanbooruAccountMutations();
  const [username, setUsername] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const busy = mutations.save.isPending || mutations.probe.isPending || mutations.remove.isPending;

  useEffect(() => {
    if (account.data?.username) setUsername(account.data.username);
  }, [account.data?.username]);

  const failure = useCallback(
    (error: unknown) =>
      pushToast({
        level: "error",
        title: t("danbooruActionFailed"),
        message: error instanceof Error ? error.message : String(error),
      }),
    [pushToast, t],
  );

  const save = useCallback(() => {
    mutations.save.mutate(
      { username: username.trim(), api_key: apiKey.trim() || null },
      {
        onSuccess: (saved) => {
          setApiKey("");
          setUsername(saved.username ?? username.trim());
          setEditorOpen(false);
          pushToast({ level: "success", message: t("danbooruSaved") });
        },
        onError: failure,
      },
    );
  }, [apiKey, failure, mutations.save, pushToast, t, username]);

  const probe = useCallback(() => {
    mutations.probe.mutate(undefined, {
      onSuccess: (status) =>
        pushToast({
          level: status.state === "verified" ? "success" : "warning",
          message:
            status.state === "verified"
              ? t("danbooruVerified", { level: status.level ?? t("unknownLevel") })
              : t("danbooruInvalid"),
        }),
      onError: failure,
    });
  }, [failure, mutations.probe, pushToast, t]);

  const remove = useCallback(() => {
    mutations.remove.mutate(undefined, {
      onSuccess: () => {
        setUsername("");
        setApiKey("");
        setConfirmDelete(false);
        pushToast({ level: "success", message: t("danbooruRemoved") });
      },
      onError: failure,
    });
  }, [failure, mutations.remove, pushToast, t]);
  const openDelete = useCallback(() => setConfirmDelete(true), []);
  const closeDelete = useCallback(() => setConfirmDelete(false), []);
  const openEditor = useCallback(() => {
    setUsername(account.data?.username ?? "");
    setApiKey("");
    setEditorOpen(true);
  }, [account.data?.username]);
  const closeEditor = useCallback(() => {
    if (!busy) setEditorOpen(false);
  }, [busy]);
  const openProfile = useCallback(() => {
    void desktopApi.openExternalUrl("https://danbooru.donmai.us/profile").catch(failure);
  }, [failure]);

  if (account.isPending) return <LoadingPanel label={t("loadingDanbooruAccount")} />;

  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <SectionHeader title={t("connections")} />
      <div className="min-h-0 flex-1 overflow-auto p-3">
        <section className="grid max-w-2xl gap-4 border border-app-border bg-app-surface/45 p-4">
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-semibold text-white">{t("danbooruServiceName")}</h2>
                <span className="border border-app-border bg-black/20 px-2 py-0.5 text-[10px] font-semibold text-app-muted uppercase">
                  {t("optional")}
                </span>
              </div>
              {account.data?.username ? (
                <p className="mt-1 text-xs text-app-muted">{account.data.username}</p>
              ) : null}
            </div>
            <span className="border border-app-border bg-black/20 px-2 py-1 text-xs text-app-muted">
              {account.data ? t(`danbooruStates.${account.data.state}`) : t("unavailable")}
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-2 border-t border-app-border pt-3">
            <AppButton variant="secondary" disabled={busy} onClick={openEditor}>
              {account.data?.configured ? (
                <PencilLine aria-hidden="true" className="size-4" />
              ) : (
                <Plus aria-hidden="true" className="size-4" />
              )}
              {account.data?.configured ? t("editDanbooru") : t("configureDanbooru")}
            </AppButton>
            <AppButton
              variant="secondary"
              disabled={busy || !account.data?.configured}
              onClick={probe}
            >
              {mutations.probe.isPending ? t("checkingDanbooru") : t("testDanbooru")}
            </AppButton>
            <AppButton
              variant="danger"
              disabled={busy || !account.data?.configured}
              onClick={openDelete}
            >
              {t("removeDanbooru")}
            </AppButton>
            <AppButton variant="ghost" className="ml-auto" onClick={openProfile}>
              <ExternalLink aria-hidden="true" className="size-4" />
              {t("openDanbooruProfile")}
            </AppButton>
          </div>
        </section>
      </div>
      <DanbooruConnectionModal
        open={editorOpen}
        configured={account.data?.configured === true}
        username={username}
        apiKey={apiKey}
        busy={busy}
        onUsernameChange={setUsername}
        onApiKeyChange={setApiKey}
        onSave={save}
        onClose={closeEditor}
      />
      <RemoveConnectionModal
        open={confirmDelete}
        busy={busy}
        onClose={closeDelete}
        onRemove={remove}
      />
    </AppPanel>
  );
}

function DanbooruConnectionModal({
  open,
  configured,
  username,
  apiKey,
  busy,
  onUsernameChange,
  onApiKeyChange,
  onSave,
  onClose,
}: {
  open: boolean;
  configured: boolean;
  username: string;
  apiKey: string;
  busy: boolean;
  onUsernameChange: (value: string) => void;
  onApiKeyChange: (value: string) => void;
  onSave: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  const saveDisabled = busy || username.trim() === "" || (!configured && apiKey.trim() === "");

  return (
    <AppModal
      open={open}
      title={configured ? t("editDanbooru") : t("configureDanbooru")}
      onClose={onClose}
    >
      <div className="grid gap-4">
        <TextField
          label={t("danbooruUsername")}
          value={username}
          onChange={onUsernameChange}
          placeholder={t("danbooruUsernamePlaceholder")}
          disabled={busy}
        />
        <TextField
          label={t("danbooruApiKey")}
          value={apiKey}
          onChange={onApiKeyChange}
          placeholder={configured ? t("danbooruKeepExistingKey") : t("danbooruApiKeyPlaceholder")}
          type="password"
          autoComplete="new-password"
          disabled={busy}
        />
        <div className="flex justify-end gap-2 border-t border-app-border pt-3">
          <AppButton variant="ghost" disabled={busy} onClick={onClose}>
            {translateCommon("cancel")}
          </AppButton>
          <AppButton disabled={saveDisabled} onClick={onSave}>
            {busy ? t("savingDanbooru") : t("saveDanbooru")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}

function RemoveConnectionModal({
  open,
  busy,
  onClose,
  onRemove,
}: {
  open: boolean;
  busy: boolean;
  onClose: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation("settings");
  return (
    <AppModal open={open} title={t("removeDanbooruTitle")} onClose={onClose}>
      <p className="text-sm text-app-muted">{t("removeDanbooruDescription")}</p>
      <div className="mt-4 flex justify-end gap-2 border-t border-app-border pt-3">
        <AppButton variant="ghost" disabled={busy} onClick={onClose}>
          {t("cancel")}
        </AppButton>
        <AppButton variant="danger" disabled={busy} onClick={onRemove}>
          {t("removeDanbooru")}
        </AppButton>
      </div>
    </AppModal>
  );
}
