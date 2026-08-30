import { Download } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";

import {
  useCompleteResourceOnboardingMutation,
  useDownloadableResourcesQuery,
  useInstallDownloadableResourceGroupMutation,
} from "../data/useDownloadableResources";
import { formatError } from "../settings-utils";

const GROUP_IDS = ["starter", "semantic-search", "enhanced-safety"] as const;

export function ResourceOnboarding() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const resources = useDownloadableResourcesQuery();
  const install = useInstallDownloadableResourceGroupMutation();
  const complete = useCompleteResourceOnboardingMutation();
  const [selected, setSelected] = useState<Set<string>>(() => new Set(["starter"]));
  const groups = useMemo(
    () => new Map(resources.data?.groups.map((group) => [group.id, group]) ?? []),
    [resources.data],
  );
  const visible = resources.data !== undefined && !resources.data.onboarding_complete;
  const finish = useCallback(async () => {
    try {
      for (const groupId of GROUP_IDS) {
        if (selected.has(groupId)) {
          await install.mutateAsync({
            request: { group_id: groupId },
            onProgress: () => undefined,
          });
        }
      }
      await complete.mutateAsync();
    } catch (error) {
      pushToast({ level: "error", title: t("resourceActionFailed"), message: formatError(error) });
    }
  }, [complete, install, pushToast, selected, t]);
  const skip = useCallback(() => {
    complete.mutate(undefined, {
      onError: (error) =>
        pushToast({
          level: "error",
          title: t("resourceActionFailed"),
          message: formatError(error),
        }),
    });
  }, [complete, pushToast, t]);
  const handleFinish = useCallback(() => void finish(), [finish]);
  const retry = useCallback(() => void resources.refetch(), [resources]);
  const toggle = useCallback((id: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  if (resources.isError) {
    return (
      <AppModal open title={t("resourceOnboardingTitle")} onClose={skip}>
        <div className="grid gap-4">
          <p className="text-sm font-semibold text-app-text">{t("resourcesUnavailable")}</p>
          <p className="text-sm leading-5 text-app-muted">{formatError(resources.error)}</p>
          <div className="flex justify-end gap-2">
            <AppButton variant="ghost" onClick={skip}>
              {t("skipResources")}
            </AppButton>
            <AppButton variant="secondary" onClick={retry}>
              {t("retryResources")}
            </AppButton>
          </div>
        </div>
      </AppModal>
    );
  }

  return (
    <AppModal open={visible} title={t("resourceOnboardingTitle")} onClose={skip}>
      <div className="grid gap-4">
        <p className="text-sm leading-5 text-app-muted">{t("resourceOnboardingDescription")}</p>
        <div className="grid gap-2">
          {GROUP_IDS.map((id) => {
            const group = groups.get(id);
            return (
              <ResourceChoice
                key={id}
                id={id}
                checked={selected.has(id)}
                title={t(`resourceGroups.${id}.title`)}
                description={t(`resourceGroups.${id}.description`)}
                size={formatBytes(group?.size_bytes ?? 0)}
                onToggle={toggle}
              />
            );
          })}
        </div>
        <div className="flex justify-end gap-2">
          <AppButton
            variant="ghost"
            disabled={complete.isPending || install.isPending}
            onClick={skip}
          >
            {t("skipResources")}
          </AppButton>
          <AppButton disabled={complete.isPending || install.isPending} onClick={handleFinish}>
            <Download aria-hidden="true" className="size-4" />
            {install.isPending ? t("installingResources") : t("installSelectedResources")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}

function ResourceChoice({
  id,
  checked,
  title,
  description,
  size,
  onToggle,
}: {
  id: string;
  checked: boolean;
  title: string;
  description: string;
  size: string;
  onToggle: (id: string) => void;
}) {
  const { t } = useTranslation("settings");
  const handleChange = useCallback(() => onToggle(id), [id, onToggle]);
  return (
    <label className="flex cursor-pointer gap-3 border border-app-border p-3">
      <input
        type="checkbox"
        aria-label={title}
        checked={checked}
        onChange={handleChange}
        className="mt-1 accent-brand-500"
      />
      <span>
        <span className="block text-sm font-semibold text-app-text">{title}</span>
        <span className="mt-1 block text-xs leading-4 text-app-muted">
          {t("resourceGroupSummary", { description, size })}
        </span>
      </span>
    </label>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  return `${Math.ceil(bytes / 1024)} KB`;
}
