/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-array-as-prop */
import { PencilLine, RefreshCw, Save, Trash2 } from "lucide-react";
import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton } from "@/components/ui";
import { agentApi } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type { AgentModelDto, DiscoveredAgentModelDto, SaveAgentModelRequestDto } from "@/types";

import { NumberField, SelectField, TextField } from "../../settings/components/SettingsControls";
import { useAgentRegistryMutations, useAgentRegistryQuery } from "../data/useAgentQueries";
import {
  DEFAULT_CONTEXT_WINDOW,
  DEFAULT_MAX_OUTPUT,
  newModelDraft,
  notifyAgentSettingsError,
  SettingsBlock,
} from "./agent-settings-shared";

export function AgentModelsSettings() {
  const { t } = useTranslation("agent");
  const pushToast = useToastStore((state) => state.push);
  const registry = useAgentRegistryQuery();
  const mutations = useAgentRegistryMutations();
  const firstConnectionId = registry.data?.connections.at(0)?.id ?? "";
  const [selectedConnectionId, setSelectedConnectionId] = useState("");
  const connectionId = selectedConnectionId || firstConnectionId;
  const [discovered, setDiscovered] = useState<DiscoveredAgentModelDto[]>([]);
  const [discovering, setDiscovering] = useState(false);
  const [draft, setDraft] = useState<SaveAgentModelRequestDto>(() => newModelDraft(""));
  const effectiveDraft = useMemo(
    () => (draft.connection_id ? draft : { ...draft, connection_id: connectionId }),
    [connectionId, draft],
  );

  const discover = useCallback(() => {
    if (!connectionId) return;
    setDiscovering(true);
    void agentApi
      .discoverModels({ connection_id: connectionId })
      .then(setDiscovered)
      .catch((error) => notifyAgentSettingsError(pushToast, t("discoveryFailed"), error))
      .finally(() => setDiscovering(false));
  }, [connectionId, pushToast, t]);

  const save = useCallback(() => {
    mutations.saveModel.mutate(effectiveDraft, {
      onSuccess: () => {
        setDraft(newModelDraft(connectionId));
        pushToast({ level: "success", message: t("modelSaved") });
      },
      onError: (error) => notifyAgentSettingsError(pushToast, t("modelSaveFailed"), error),
    });
  }, [connectionId, effectiveDraft, mutations.saveModel, pushToast, t]);

  if (!registry.data?.connections.length) {
    return (
      <SettingsBlock title={t("modelsTitle")} description={t("modelsDescription")}>
        <p className="text-xs text-app-muted">{t("connectionRequired")}</p>
      </SettingsBlock>
    );
  }

  return (
    <SettingsBlock title={t("modelsTitle")} description={t("modelsDescription")}>
      <ModelRows
        models={registry.data.models}
        onEdit={setDraft}
        onDelete={(id) =>
          mutations.deleteModel.mutate(
            { id },
            {
              onError: (error) =>
                notifyAgentSettingsError(pushToast, t("modelDeleteFailed"), error),
            },
          )
        }
      />
      <div className="flex items-end gap-2 border-t border-app-border pt-4">
        <SelectField
          label={t("connection")}
          value={connectionId}
          options={registry.data.connections.map((connection) => ({
            value: connection.id,
            label: connection.display_name,
          }))}
          onChange={(value) => {
            setSelectedConnectionId(value);
            setDiscovered([]);
            setDraft(newModelDraft(value));
          }}
        />
        <AppButton variant="secondary" disabled={discovering} onClick={discover}>
          <RefreshCw
            aria-hidden="true"
            className={discovering ? "size-4 animate-spin" : "size-4"}
          />
          {t("discoverModels")}
        </AppButton>
      </div>
      <DiscoveredModels models={discovered} connectionId={connectionId} onSelect={setDraft} />
      <ModelEditor draft={effectiveDraft} setDraft={setDraft} />
      <div className="flex justify-end">
        <AppButton
          disabled={
            mutations.saveModel.isPending ||
            !effectiveDraft.connection_id ||
            !effectiveDraft.wire_model_id.trim() ||
            !effectiveDraft.display_name.trim()
          }
          onClick={save}
        >
          <Save aria-hidden="true" className="size-4" />
          {t("saveModel")}
        </AppButton>
      </div>
    </SettingsBlock>
  );
}

function ModelRows({
  models,
  onEdit,
  onDelete,
}: {
  models: AgentModelDto[];
  onEdit: (model: SaveAgentModelRequestDto) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation("agent");
  return (
    <div className="grid gap-2">
      {models.map((model) => (
        <div
          key={model.id}
          className="flex items-center gap-3 border border-app-border bg-black/15 px-3 py-2"
        >
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-white">{model.display_name}</p>
            <p className="truncate text-xs text-app-muted">
              {model.wire_model_id} · {model.context_window.toLocaleString()} /{" "}
              {model.max_output_tokens.toLocaleString()}
            </p>
          </div>
          <AppIconButton
            icon={PencilLine}
            label={t("editModel")}
            size="sm"
            onClick={() => onEdit(model)}
          />
          <AppIconButton
            icon={Trash2}
            label={t("deleteModel")}
            size="sm"
            variant="danger"
            onClick={() => onDelete(model.id)}
          />
        </div>
      ))}
    </div>
  );
}

function DiscoveredModels({
  models,
  connectionId,
  onSelect,
}: {
  models: DiscoveredAgentModelDto[];
  connectionId: string;
  onSelect: (model: SaveAgentModelRequestDto) => void;
}) {
  if (!models.length) return null;
  return (
    <div className="flex flex-wrap gap-2">
      {models.map((model) => (
        <button
          key={model.wire_model_id}
          type="button"
          className="border border-app-border bg-black/15 px-2 py-1 text-xs text-app-muted hover:border-brand-400 hover:text-white"
          onClick={() =>
            onSelect({
              ...newModelDraft(connectionId),
              wire_model_id: model.wire_model_id,
              display_name: model.display_name,
              context_window: model.context_window ?? DEFAULT_CONTEXT_WINDOW,
              max_output_tokens: model.max_output_tokens ?? DEFAULT_MAX_OUTPUT,
            })
          }
        >
          {model.display_name}
        </button>
      ))}
    </div>
  );
}

function ModelEditor({
  draft,
  setDraft,
}: {
  draft: SaveAgentModelRequestDto;
  setDraft: Dispatch<SetStateAction<SaveAgentModelRequestDto>>;
}) {
  const { t } = useTranslation("agent");
  return (
    <div className="grid gap-3 md:grid-cols-2">
      <TextField
        label={t("wireModelId")}
        value={draft.wire_model_id}
        onChange={(wire_model_id) => setDraft((current) => ({ ...current, wire_model_id }))}
      />
      <TextField
        label={t("modelDisplayName")}
        value={draft.display_name}
        onChange={(display_name) => setDraft((current) => ({ ...current, display_name }))}
      />
      <NumberField
        label={t("contextWindow")}
        value={draft.context_window}
        onChange={(context_window) => setDraft((current) => ({ ...current, context_window }))}
      />
      <NumberField
        label={t("maxOutputTokens")}
        value={draft.max_output_tokens}
        onChange={(max_output_tokens) => setDraft((current) => ({ ...current, max_output_tokens }))}
      />
      <NumberField
        label={t("temperature")}
        value={draft.temperature}
        step="0.1"
        onChange={(temperature) => setDraft((current) => ({ ...current, temperature }))}
      />
    </div>
  );
}
