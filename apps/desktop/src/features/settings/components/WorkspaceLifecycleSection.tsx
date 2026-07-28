import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import type { WorkspaceStatusDto } from "@/types";

import { SectionHeader } from "./SettingsControls";

export function WorkspaceLifecycleSection({
  workspace,
  closeWorkspace,
  closing,
}: {
  workspace: WorkspaceStatusDto;
  closeWorkspace: () => void;
  closing: boolean;
}) {
  const { t } = useTranslation("settings");
  return (
    <AppPanel variant="section" className="h-full min-h-0 overflow-hidden">
      <SectionHeader title={t("workspace")}>
        <AppButton onClick={closeWorkspace} disabled={closing}>
          {closing ? t("closingWorkspace") : t("closeWorkspace")}
        </AppButton>
      </SectionHeader>
      <dl className="p-3 text-sm">
        <div className="border border-app-border bg-app-surface p-3">
          <dt className="flex items-center gap-2 text-xs font-semibold text-app-muted uppercase">
            <FolderOpen aria-hidden="true" className="size-4" /> {t("root")}
          </dt>
          <dd className="mt-2 break-all text-app-text">{workspace.root}</dd>
        </div>
      </dl>
    </AppPanel>
  );
}
