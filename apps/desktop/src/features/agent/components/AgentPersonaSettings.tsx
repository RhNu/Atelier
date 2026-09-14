/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { PencilLine, Save } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { AgentModelDto, AgentPermissionModeDto, AgentWorkspaceSettingsDto } from "@/types";

import { SelectField, TextField } from "../../settings/components/SettingsControls";
import { useUpdateAgentWorkspaceSettingsMutation } from "../data/useAgentQueries";
import { notifyAgentSettingsError, SettingsBlock, TextAreaField } from "./agent-settings-shared";

export function AgentPersonaSettings({
  initial,
  modelOptions,
}: {
  initial: AgentWorkspaceSettingsDto;
  modelOptions: AgentModelDto[];
}) {
  const { t } = useTranslation("agent");
  const pushToast = useToastStore((state) => state.push);
  const mutation = useUpdateAgentWorkspaceSettingsMutation();
  const [draft, setDraft] = useState<AgentWorkspaceSettingsDto | null>(null);
  const defaultModelOptions = useMemo(
    () => [
      { value: "", label: t("firstAvailableModel") },
      ...modelOptions.map((model) => ({ value: model.id, label: model.display_name })),
    ],
    [modelOptions, t],
  );
  const currentModel =
    modelOptions.find((model) => model.id === initial.default_model_id)?.display_name ??
    t("firstAvailableModel");
  const save = useCallback(() => {
    if (!draft) return;
    mutation.mutate(
      { settings: draft },
      {
        onSuccess: () => {
          setDraft(null);
          pushToast({ level: "success", message: t("personaSaved") });
        },
        onError: (error) => notifyAgentSettingsError(pushToast, t("personaSaveFailed"), error),
      },
    );
  }, [draft, mutation, pushToast, t]);

  return (
    <SettingsBlock title={t("personaTitle")} description={t("personaDescription")}>
      <div className="flex items-center gap-3 border border-app-border bg-black/15 px-3 py-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold text-white">{initial.display_name}</p>
          <p className="truncate text-xs text-app-muted">
            {t("personaSummary", {
              model: currentModel,
              permission: permissionLabel(initial.permission_mode, t),
            })}
          </p>
        </div>
        <AppButton variant="secondary" onClick={() => setDraft(structuredClone(initial))}>
          <PencilLine aria-hidden="true" className="size-4" />
          {t("editPersona")}
        </AppButton>
      </div>
      <AppModal
        open={draft !== null}
        title={t("editPersona")}
        density="compact"
        onClose={() => setDraft(null)}
      >
        {draft ? (
          <PersonaEditor
            draft={draft}
            modelOptions={defaultModelOptions}
            busy={mutation.isPending}
            onChange={setDraft}
            onCancel={() => setDraft(null)}
            onSave={save}
          />
        ) : null}
      </AppModal>
    </SettingsBlock>
  );
}

function PersonaEditor({
  draft,
  modelOptions,
  busy,
  onChange,
  onCancel,
  onSave,
}: {
  draft: AgentWorkspaceSettingsDto;
  modelOptions: ReadonlyArray<{ value: string; label: string }>;
  busy: boolean;
  onChange: (draft: AgentWorkspaceSettingsDto) => void;
  onCancel: () => void;
  onSave: () => void;
}) {
  const { t } = useTranslation("agent");
  const update = (change: Partial<AgentWorkspaceSettingsDto>) => onChange({ ...draft, ...change });
  const selectedModelId = modelOptions.some((model) => model.value === draft.default_model_id)
    ? (draft.default_model_id ?? "")
    : "";
  return (
    <div className="grid gap-3">
      <TextField
        label={t("personaName")}
        value={draft.display_name}
        onChange={(display_name) => update({ display_name })}
      />
      <TextAreaField
        label={t("instructions")}
        value={draft.instructions}
        onChange={(instructions) => update({ instructions })}
      />
      <TextAreaField
        label={t("v5Guidance")}
        value={draft.v5_prompt_guidance}
        onChange={(v5_prompt_guidance) => update({ v5_prompt_guidance })}
      />
      <TextAreaField
        label={t("tagGuidance")}
        value={draft.tag_prompt_guidance}
        onChange={(tag_prompt_guidance) => update({ tag_prompt_guidance })}
      />
      <SelectField
        label={t("defaultModel")}
        value={selectedModelId}
        options={modelOptions}
        onChange={(default_model_id) => update({ default_model_id: default_model_id || null })}
      />
      <div className="flex justify-end gap-2">
        <AppButton variant="ghost" disabled={busy} onClick={onCancel}>
          {t("cancel")}
        </AppButton>
        <AppButton disabled={busy || !draft.display_name.trim()} onClick={onSave}>
          <Save aria-hidden="true" className="size-4" />
          {t("savePersona")}
        </AppButton>
      </div>
    </div>
  );
}

function permissionLabel(
  mode: AgentPermissionModeDto,
  t: ReturnType<typeof useTranslation<"agent">>["t"],
) {
  if (mode === "ask") return t("permissionAsk");
  if (mode === "bypass_all") return t("permissionBypass");
  return t("permissionStandard");
}
