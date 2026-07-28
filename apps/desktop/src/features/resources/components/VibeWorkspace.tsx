/* eslint-disable max-lines-per-function, react-perf/jsx-no-new-function-as-prop */
import { Download, FilePlus2, Import, Save } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { runLoggedAction } from "@/app/logger";
import { AppButton, AppModal, AppPanel, EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { VibeDocumentEntryDto } from "@/types";

import {
  useEnsureVibeEncodingFromSourceMutation,
  useExportVibeDocumentMutation,
  useImportEmbeddedPngVibeDocumentsMutation,
  useImportVibeDocumentsMutation,
  useRenameVibeDocumentMutation,
  useSetVibeDocumentHiddenMutation,
} from "../data/useResourcesData";
import { formatError, matchesSearch } from "../resource-model";
import type { ResourceViewMode } from "../resource-model";
import { CheckboxField, TextInput } from "./ResourceEditorPrimitives";
import { VibeCard } from "./VibeCard";

export function VibeWorkspace({
  vibes,
  pending,
  error,
  search,
  includeHidden,
  onIncludeHiddenChange,
  viewMode,
}: {
  vibes: ReadonlyArray<VibeDocumentEntryDto>;
  pending: boolean;
  error: string | null;
  search: string;
  includeHidden: boolean;
  onIncludeHiddenChange: (value: boolean) => void;
  viewMode: ResourceViewMode;
}) {
  const { t } = useTranslation("resources");
  const pushToast = useToastStore((state) => state.push);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [editingVibe, setEditingVibe] = useState<VibeDocumentEntryDto | null>(null);
  const filtered = useMemo(
    () => vibes.filter((vibe) => matchesSearch(search, vibe.display_name, vibe.vibe_id)),
    [search, vibes],
  );
  const importVibeMutation = useImportVibeDocumentsMutation();
  const importPngMutation = useImportEmbeddedPngVibeDocumentsMutation();
  const ensureEncodingMutation = useEnsureVibeEncodingFromSourceMutation();
  const exportMutation = useExportVibeDocumentMutation();
  const renameMutation = useRenameVibeDocumentMutation();
  const hideMutation = useSetVibeDocumentHiddenMutation();
  const commandError =
    importVibeMutation.error ??
    importPngMutation.error ??
    ensureEncodingMutation.error ??
    exportMutation.error ??
    renameMutation.error ??
    hideMutation.error;
  useEffect(() => {
    if (commandError) {
      pushToast({
        level: "error",
        title: t("vibeActionFailed"),
        message: formatError(commandError),
      });
    }
  }, [commandError, pushToast, t]);

  function exportSelected() {
    if (selectedIds.length === 0) {
      return;
    }
    exportMutation.mutate({
      vibe_ids: selectedIds,
      format: selectedIds.length === 1 ? "naiv4vibe" : "naiv4vibebundle",
    });
  }
  function ensureEncodingHandler(vibe: VibeDocumentEntryDto) {
    const sourceImage = vibe.source_image;
    return sourceImage
      ? () =>
          ensureEncodingMutation.mutate({
            vibeId: vibe.vibe_id,
            sourceImage,
          })
      : null;
  }

  return (
    <AppPanel variant="section" className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <div>
          <h2 className="text-sm font-semibold text-white">{t("vibeDocuments")}</h2>
          <p className="text-xs text-app-muted">{t("vibeDescription")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <CheckboxField
            label={t("showHidden")}
            checked={includeHidden}
            onChange={onIncludeHiddenChange}
          />
          <AppButton
            variant="secondary"
            onClick={() => importVibeMutation.mutate()}
            disabled={importVibeMutation.isPending}
          >
            <Import aria-hidden="true" className="size-4" />
            {t("importVibeDocument")}
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={() => importPngMutation.mutate()}
            disabled={importPngMutation.isPending}
          >
            <FilePlus2 aria-hidden="true" className="size-4" />
            {t("importPngVibe")}
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={exportSelected}
            disabled={exportMutation.isPending || selectedIds.length === 0}
          >
            <Download aria-hidden="true" className="size-4" />
            {t("exportSelected")}
          </AppButton>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {pending ? (
          <EmptyState title={t("loadingVibes")} />
        ) : error ? (
          <EmptyState title={t("vibeUnavailable")} description={error} />
        ) : filtered.length === 0 ? (
          <EmptyState title={t("noVibes")} iconOnly />
        ) : (
          <div
            className={
              viewMode === "grid"
                ? "grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3"
                : "grid gap-1"
            }
          >
            {filtered.map((vibe) => (
              <VibeCard
                key={vibe.vibe_id}
                vibe={vibe}
                viewMode={viewMode}
                selected={selectedIds.includes(vibe.vibe_id)}
                exportPending={exportMutation.isPending}
                encodePending={ensureEncodingMutation.isPending}
                onToggleSelected={(selected) =>
                  setSelectedIds((current) =>
                    selected
                      ? [...current, vibe.vibe_id]
                      : current.filter((id) => id !== vibe.vibe_id),
                  )
                }
                onExport={() =>
                  exportMutation.mutate({
                    vibe_ids: [vibe.vibe_id],
                    format: "naiv4vibe",
                  })
                }
                onEdit={() => setEditingVibe(vibe)}
                onEnsureEncoding={ensureEncodingHandler(vibe)}
              />
            ))}
          </div>
        )}
      </div>
      <VibeEditDialog
        vibe={editingVibe}
        saving={renameMutation.isPending || hideMutation.isPending}
        error={null}
        onClose={() => setEditingVibe(null)}
        onSave={(displayName, hidden) => {
          if (!editingVibe) return;
          const updates: Promise<unknown>[] = [];
          if (displayName !== editingVibe.display_name) {
            updates.push(
              renameMutation.mutateAsync({
                vibe_id: editingVibe.vibe_id,
                display_name: displayName,
              }),
            );
          }
          if (hidden !== editingVibe.hidden) {
            updates.push(hideMutation.mutateAsync({ vibe_id: editingVibe.vibe_id, hidden }));
          }
          void runLoggedAction("Update Vibe document", () =>
            Promise.all(updates).then(() => setEditingVibe(null)),
          ).catch((error: unknown) => {
            pushToast({
              level: "error",
              title: t("vibeActionFailed"),
              message: formatError(error),
            });
          });
        }}
      />
    </AppPanel>
  );
}

function VibeEditDialog({
  vibe,
  saving,
  error,
  onClose,
  onSave,
}: {
  vibe: VibeDocumentEntryDto | null;
  saving: boolean;
  error: string | null;
  onClose: () => void;
  onSave: (displayName: string, hidden: boolean) => void;
}) {
  const { t } = useTranslation("resources");
  const [name, setName] = useState("");
  const [hidden, setHidden] = useState(false);
  useEffect(() => {
    setName(vibe?.display_name ?? "");
    setHidden(vibe?.hidden ?? false);
  }, [vibe?.display_name, vibe?.hidden, vibe?.vibe_id]);
  const close = () => {
    setName("");
    onClose();
  };
  return (
    <AppModal open={Boolean(vibe)} title={t("editVibe")} onClose={close}>
      <div className="grid gap-4">
        {error ? (
          <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
            {error}
          </p>
        ) : null}
        <TextInput label={t("localDisplayName")} value={name} onChange={setName} />
        <label className="flex h-9 items-center gap-2 border border-app-border bg-black/20 px-3 text-sm text-app-text">
          <input
            aria-label={t("hidden")}
            type="checkbox"
            checked={hidden}
            onChange={(event) => setHidden(event.target.checked)}
          />
          {t("hidden")}
        </label>
        <AppButton onClick={() => onSave(name, hidden)} disabled={saving || !name.trim()}>
          <Save aria-hidden="true" className="size-4" />
          {t("saveChanges")}
        </AppButton>
      </div>
    </AppModal>
  );
}
