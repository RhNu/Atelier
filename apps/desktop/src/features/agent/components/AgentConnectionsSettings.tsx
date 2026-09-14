/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-array-as-prop */
import { AlertTriangle, PencilLine, Plus, Save, Trash2 } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { AgentAuthKindDto, SaveAgentConnectionRequestDto } from "@/types";

import { SelectField, TextField } from "../../settings/components/SettingsControls";
import { useAgentRegistryMutations, useAgentRegistryQuery } from "../data/useAgentQueries";
import {
  newConnectionDraft,
  notifyAgentSettingsError,
  SettingsBlock,
} from "./agent-settings-shared";

export function AgentConnectionsSettings() {
  const { t } = useTranslation("agent");
  const pushToast = useToastStore((state) => state.push);
  const registry = useAgentRegistryQuery();
  const mutations = useAgentRegistryMutations();
  const [draft, setDraft] = useState<SaveAgentConnectionRequestDto>(newConnectionDraft);
  const busy = mutations.saveConnection.isPending || mutations.deleteConnection.isPending;
  const save = useCallback(() => {
    mutations.saveConnection.mutate(
      { ...draft, display_name: draft.display_name.trim(), base_url: draft.base_url.trim() },
      {
        onSuccess: () => {
          setDraft(newConnectionDraft());
          pushToast({ level: "success", message: t("connectionSaved") });
        },
        onError: (error) => notifyAgentSettingsError(pushToast, t("connectionSaveFailed"), error),
      },
    );
  }, [draft, mutations.saveConnection, pushToast, t]);
  const changeAuth = (value: string) =>
    setDraft((current) => ({ ...current, auth_kind: parseAuthKind(value) }));

  return (
    <SettingsBlock title={t("connectionsTitle")} description={t("connectionsDescription")}>
      <div className="grid gap-2">
        {registry.data?.connections.map((connection) => (
          <div
            key={connection.id}
            className="flex items-center gap-3 border border-app-border bg-black/15 px-3 py-2"
          >
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-semibold text-white">{connection.display_name}</p>
              <p className="truncate text-xs text-app-muted">{connection.base_url}</p>
              {connection.insecure_remote_http ? (
                <p className="mt-1 flex items-center gap-1 text-[11px] text-amber-200">
                  <AlertTriangle aria-hidden="true" className="size-3" />
                  {t("insecureConnection")}
                </p>
              ) : null}
            </div>
            <AppIconButton
              icon={PencilLine}
              label={t("editConnection")}
              size="sm"
              disabled={busy}
              onClick={() =>
                setDraft({
                  id: connection.id,
                  display_name: connection.display_name,
                  base_url: connection.base_url,
                  auth_kind: connection.auth_kind,
                  secret: null,
                })
              }
            />
            <AppIconButton
              icon={Trash2}
              label={t("deleteConnection")}
              size="sm"
              variant="danger"
              disabled={busy}
              onClick={() =>
                mutations.deleteConnection.mutate(
                  { id: connection.id },
                  {
                    onError: (error) =>
                      notifyAgentSettingsError(pushToast, t("connectionDeleteFailed"), error),
                  },
                )
              }
            />
          </div>
        ))}
      </div>
      <div className="grid gap-3 border-t border-app-border pt-4 md:grid-cols-2">
        <TextField
          label={t("connectionName")}
          value={draft.display_name}
          onChange={(display_name) => setDraft((current) => ({ ...current, display_name }))}
        />
        <TextField
          label={t("baseUrl")}
          value={draft.base_url}
          placeholder={t("baseUrlPlaceholder")}
          onChange={(base_url) => setDraft((current) => ({ ...current, base_url }))}
        />
        <SelectField
          label={t("authentication")}
          value={draft.auth_kind}
          options={[
            { value: "none", label: t("authNone") },
            { value: "bearer", label: t("authBearer") },
          ]}
          onChange={changeAuth}
        />
        <TextField
          label={t("apiKey")}
          value={draft.secret ?? ""}
          type="password"
          disabled={draft.auth_kind === "none"}
          placeholder={t("apiKeyPlaceholder")}
          onChange={(secret) => setDraft((current) => ({ ...current, secret: secret || null }))}
        />
      </div>
      <div className="flex justify-end gap-2">
        <AppButton variant="ghost" disabled={busy} onClick={() => setDraft(newConnectionDraft())}>
          <Plus aria-hidden="true" className="size-4" />
          {t("newConnection")}
        </AppButton>
        <AppButton
          disabled={busy || !draft.display_name.trim() || !draft.base_url.trim()}
          onClick={save}
        >
          <Save aria-hidden="true" className="size-4" />
          {t("saveConnection")}
        </AppButton>
      </div>
    </SettingsBlock>
  );
}

function parseAuthKind(value: string): AgentAuthKindDto {
  return value === "none" ? "none" : "bearer";
}
