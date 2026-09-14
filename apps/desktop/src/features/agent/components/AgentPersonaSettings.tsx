/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-array-as-prop */
import { AlertTriangle, Save } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
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
  const [draft, setDraft] = useState(() => structuredClone(initial));
  const defaultModelOptions = useMemo(
    () => [
      { value: "", label: t("firstAvailableModel") },
      ...modelOptions.map((model) => ({ value: model.id, label: model.display_name })),
    ],
    [modelOptions, t],
  );
  const selectedModelId = modelOptions.some((model) => model.id === draft.default_model_id)
    ? (draft.default_model_id ?? "")
    : "";
  const save = useCallback(() => {
    mutation.mutate(
      { settings: draft },
      {
        onSuccess: (saved) => {
          setDraft(structuredClone(saved));
          pushToast({ level: "success", message: t("personaSaved") });
        },
        onError: (error) => notifyAgentSettingsError(pushToast, t("personaSaveFailed"), error),
      },
    );
  }, [draft, mutation, pushToast, t]);
  const changePermission = (value: string) =>
    setDraft((current) => ({ ...current, permission_mode: parsePermissionMode(value) }));

  return (
    <SettingsBlock title={t("personaTitle")} description={t("personaDescription")}>
      <TextField
        label={t("personaName")}
        value={draft.display_name}
        onChange={(display_name) => setDraft((current) => ({ ...current, display_name }))}
      />
      <TextAreaField
        label={t("instructions")}
        value={draft.instructions}
        onChange={(instructions) => setDraft((current) => ({ ...current, instructions }))}
      />
      <TextAreaField
        label={t("v5Guidance")}
        value={draft.v5_prompt_guidance}
        onChange={(v5_prompt_guidance) =>
          setDraft((current) => ({ ...current, v5_prompt_guidance }))
        }
      />
      <TextAreaField
        label={t("tagGuidance")}
        value={draft.tag_prompt_guidance}
        onChange={(tag_prompt_guidance) =>
          setDraft((current) => ({ ...current, tag_prompt_guidance }))
        }
      />
      <div className="grid gap-3 md:grid-cols-2">
        <SelectField
          label={t("permissionMode")}
          value={draft.permission_mode}
          options={[
            { value: "standard", label: t("permissionStandard") },
            { value: "ask", label: t("permissionAsk") },
            { value: "bypass_all", label: t("permissionBypass") },
          ]}
          onChange={changePermission}
        />
        <SelectField
          label={t("defaultModel")}
          value={selectedModelId}
          options={defaultModelOptions}
          onChange={(default_model_id) =>
            setDraft((current) => ({ ...current, default_model_id: default_model_id || null }))
          }
        />
      </div>
      {draft.permission_mode === "bypass_all" ? (
        <p className="flex gap-2 border border-amber-500/40 bg-amber-500/10 p-3 text-xs text-amber-100">
          <AlertTriangle aria-hidden="true" className="size-4 shrink-0" />
          {t("bypassWarning")}
        </p>
      ) : null}
      <div className="flex justify-end">
        <AppButton disabled={mutation.isPending || !draft.display_name.trim()} onClick={save}>
          <Save aria-hidden="true" className="size-4" />
          {t("savePersona")}
        </AppButton>
      </div>
    </SettingsBlock>
  );
}

function parsePermissionMode(value: string): AgentPermissionModeDto {
  if (value === "ask") return "ask";
  if (value === "bypass_all") return "bypass_all";
  return "standard";
}
