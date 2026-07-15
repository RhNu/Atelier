import { FolderOpen } from "lucide-react";

import { AppButton, AppPanel } from "../../../components/ui";
import type { WorkspaceStatusDto } from "../../../types";
import { SectionHeader } from "./SettingsControls";

export function WorkspaceLifecycleSection({
  workspace,
  closeWorkspace,
  closing,
  commandError,
}: {
  workspace: WorkspaceStatusDto;
  closeWorkspace: () => void;
  closing: boolean;
  commandError: string | null;
}) {
  return (
    <AppPanel className="h-full min-h-0 overflow-hidden">
      <SectionHeader
        kicker="Workspace"
        title="Current Workspace"
        description="Inspect or close the active NovelAI creative workspace. Atelier will remember it for the next launch."
      >
        <AppButton onClick={closeWorkspace} disabled={closing}>
          {closing ? "Closing workspace" : "Close workspace"}
        </AppButton>
      </SectionHeader>
      {commandError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {commandError}
        </p>
      ) : null}
      <dl className="grid gap-3 p-3 text-sm md:grid-cols-2">
        <div className="border border-app-border bg-app-surface p-3 md:col-span-2">
          <dt className="flex items-center gap-2 text-xs font-semibold text-app-muted uppercase">
            <FolderOpen aria-hidden="true" className="size-4" /> Root
          </dt>
          <dd className="mt-2 break-all text-app-text">{workspace.root}</dd>
        </div>
        <div className="border border-app-border bg-app-surface p-3">
          <dt className="text-xs font-semibold text-app-muted uppercase">Schema version</dt>
          <dd className="mt-2 text-app-text">{workspace.schema_version}</dd>
        </div>
        <div className="border border-app-border bg-app-surface p-3">
          <dt className="text-xs font-semibold text-app-muted uppercase">Workspace lock</dt>
          <dd className="mt-2 text-app-text">{workspace.locked ? "Active" : "Not active"}</dd>
        </div>
      </dl>
    </AppPanel>
  );
}
