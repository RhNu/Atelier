import { ExternalLink } from "lucide-react";
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
              <h2 className="text-sm font-semibold text-white">{t("danbooruServiceName")}</h2>
              <p className="mt-1 text-xs text-app-muted">{t("danbooruConnectionDescription")}</p>
            </div>
            <span className="border border-app-border bg-black/20 px-2 py-1 text-xs text-app-muted">
              {account.data ? t(`danbooruStates.${account.data.state}`) : t("unavailable")}
            </span>
          </div>
          <TextField
            label={t("danbooruUsername")}
            value={username}
            onChange={setUsername}
            placeholder={t("danbooruUsernamePlaceholder")}
            disabled={busy}
          />
          <TextField
            label={t("danbooruApiKey")}
            value={apiKey}
            onChange={setApiKey}
            placeholder={
              account.data?.configured
                ? t("danbooruKeepExistingKey")
                : t("danbooruApiKeyPlaceholder")
            }
            type="password"
            autoComplete="new-password"
            disabled={busy}
          />
          <p className="text-xs text-app-muted">{t("danbooruKeyringNote")}</p>
          <div className="flex flex-wrap items-center gap-2 border-t border-app-border pt-3">
            <AppButton
              disabled={
                busy ||
                username.trim() === "" ||
                (!account.data?.configured && apiKey.trim() === "")
              }
              onClick={save}
            >
              {mutations.save.isPending ? t("savingDanbooru") : t("saveDanbooru")}
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
      <RemoveConnectionModal
        open={confirmDelete}
        busy={busy}
        onClose={closeDelete}
        onRemove={remove}
      />
    </AppPanel>
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
