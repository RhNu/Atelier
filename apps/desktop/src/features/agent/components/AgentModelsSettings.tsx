/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-array-as-prop */
import { PencilLine, Plus, RefreshCw, Save, Trash2 } from "lucide-react";
import { useCallback, useState, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, AppModal } from "@/components/ui";
import { agentApi } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type {
  AgentImageInputModeDto,
  AgentModelDto,
  DiscoveredAgentModelDto,
  SaveAgentModelRequestDto,
} from "@/types";

import { NumberField, SelectField, TextField } from "../../settings/components/SettingsControls";
import { useAgentRegistryMutations, useAgentRegistryQuery } from "../data/useAgentQueries";
import {
  DEFAULT_CONTEXT_WINDOW,
  DEFAULT_MAX_OUTPUT,
  newModelDraft,
  notifyAgentSettingsError,
  SettingsBlock,
} from "./agent-settings-shared";

type ModelEditorState = { draft: SaveAgentModelRequestDto; editing: boolean };

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
  const [editor, setEditor] = useState<ModelEditorState | null>(null);
  const busy = mutations.saveModel.isPending || mutations.deleteModel.isPending;

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
    if (!editor) return;
    mutations.saveModel.mutate(editor.draft, {
      onSuccess: () => {
        setEditor(null);
        pushToast({ level: "success", message: t("modelSaved") });
      },
      onError: (error) => notifyAgentSettingsError(pushToast, t("modelSaveFailed"), error),
    });
  }, [editor, mutations.saveModel, pushToast, t]);

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
        busy={busy}
        onEdit={(draft) => setEditor({ draft, editing: true })}
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
      <div className="flex flex-wrap items-end gap-2">
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
          }}
        />
        <AppButton variant="secondary" disabled={discovering} onClick={discover}>
          <RefreshCw
            aria-hidden="true"
            className={discovering ? "size-4 animate-spin" : "size-4"}
          />
          {t("discoverModels")}
        </AppButton>
        <AppButton
          variant="secondary"
          disabled={busy}
          onClick={() => setEditor({ draft: newModelDraft(connectionId), editing: false })}
        >
          <Plus aria-hidden="true" className="size-4" />
          {t("newModel")}
        </AppButton>
      </div>
      <DiscoveredModels
        models={discovered}
        connectionId={connectionId}
        onSelect={(draft) => setEditor({ draft, editing: false })}
      />
      <AppModal
        open={editor !== null}
        title={editor?.editing ? t("modelEditorEdit") : t("modelEditorNew")}
        density="compact"
        onClose={() => setEditor(null)}
      >
        {editor ? (
          <ModelEditor
            draft={editor.draft}
            setDraft={(next) =>
              setEditor((current) =>
                current
                  ? {
                      ...current,
                      draft: typeof next === "function" ? next(current.draft) : next,
                    }
                  : null,
              )
            }
            busy={busy}
            onCancel={() => setEditor(null)}
            onSave={save}
          />
        ) : null}
      </AppModal>
    </SettingsBlock>
  );
}

function ModelRows({
  models,
  busy,
  onEdit,
  onDelete,
}: {
  models: AgentModelDto[];
  busy: boolean;
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
            disabled={busy}
            onClick={() => onEdit(model)}
          />
          <AppIconButton
            icon={Trash2}
            label={t("deleteModel")}
            size="sm"
            variant="danger"
            disabled={busy}
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
  busy,
  onCancel,
  onSave,
}: {
  draft: SaveAgentModelRequestDto;
  setDraft: Dispatch<SetStateAction<SaveAgentModelRequestDto>>;
  busy: boolean;
  onCancel: () => void;
  onSave: () => void;
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
      <SelectField
        label={t("imageInputMode")}
        value={draft.capabilities.image_input}
        options={[
          { value: "none", label: t("imageInputOptions.none") },
          { value: "message", label: t("imageInputOptions.message") },
          { value: "tool_result", label: t("imageInputOptions.tool_result") },
        ]}
        onChange={(image_input) =>
          setDraft((current) => ({
            ...current,
            capabilities: { image_input: parseImageInputMode(image_input) },
          }))
        }
      />
      <p className="text-xs text-app-muted md:col-span-2">{t("imageInputDescription")}</p>
      <div className="flex items-end justify-end gap-2 md:col-span-2">
        <AppButton variant="ghost" disabled={busy} onClick={onCancel}>
          {t("cancel")}
        </AppButton>
        <AppButton
          disabled={
            busy ||
            !draft.connection_id ||
            !draft.wire_model_id.trim() ||
            !draft.display_name.trim()
          }
          onClick={onSave}
        >
          <Save aria-hidden="true" className="size-4" />
          {t("saveModel")}
        </AppButton>
      </div>
    </div>
  );
}

function parseImageInputMode(value: string): AgentImageInputModeDto {
  if (value === "message" || value === "tool_result") return value;
  return "none";
}
