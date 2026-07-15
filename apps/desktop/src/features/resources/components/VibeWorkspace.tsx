/* eslint-disable max-lines-per-function, react-perf/jsx-no-new-function-as-prop */
import { Download, FilePlus2, Import, Pencil, Save, Sparkles } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppPanel, EmptyState } from "@/components/ui";
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
import { CheckboxField, PreviewSlot, TextInput } from "./ResourceEditorPrimitives";

export function VibeWorkspace({
  vibes,
  pending,
  error,
  search,
  includeHidden,
  onIncludeHiddenChange,
}: {
  vibes: ReadonlyArray<VibeDocumentEntryDto>;
  pending: boolean;
  error: string | null;
  search: string;
  includeHidden: boolean;
  onIncludeHiddenChange: (value: boolean) => void;
}) {
  const { t } = useTranslation("resources");
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
            label="Show hidden"
            checked={includeHidden}
            onChange={onIncludeHiddenChange}
          />
          <AppButton
            variant="secondary"
            onClick={() => importVibeMutation.mutate()}
            disabled={importVibeMutation.isPending}
          >
            <Import aria-hidden="true" className="size-4" />
            Import .naiv4vibe
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={() => importPngMutation.mutate()}
            disabled={importPngMutation.isPending}
          >
            <FilePlus2 aria-hidden="true" className="size-4" />
            Import PNG Vibe
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={exportSelected}
            disabled={exportMutation.isPending || selectedIds.length === 0}
          >
            <Download aria-hidden="true" className="size-4" />
            Export selected
          </AppButton>
        </div>
      </header>
      {commandError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {formatError(commandError)}
        </p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {pending ? (
          <EmptyState title={t("loadingVibes")} />
        ) : error ? (
          <EmptyState title={t("vibeUnavailable")} description={error} />
        ) : filtered.length === 0 ? (
          <EmptyState title={t("noVibes")} />
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-3">
            {filtered.map((vibe) => (
              <VibeCard
                key={vibe.vibe_id}
                vibe={vibe}
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
        error={commandError ? formatError(commandError) : null}
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
          void Promise.all(updates)
            .then(() => setEditingVibe(null))
            .catch(() => undefined);
        }}
      />
    </AppPanel>
  );
}

function VibeCard({
  vibe,
  selected,
  exportPending,
  encodePending,
  onToggleSelected,
  onExport,
  onEdit,
  onEnsureEncoding,
}: {
  vibe: VibeDocumentEntryDto;
  selected: boolean;
  exportPending: boolean;
  encodePending: boolean;
  onToggleSelected: (selected: boolean) => void;
  onExport: () => void;
  onEdit: () => void;
  onEnsureEncoding: (() => void) | null;
}) {
  const { t } = useTranslation("resources");
  return (
    <article className="grid gap-3 border border-app-border bg-app-surface p-3">
      <PreviewSlot resource={vibe.preview ?? vibe.source_image} label={vibe.display_name} />
      <label className="flex items-center gap-2 text-xs text-app-muted">
        <input
          aria-label={`Select ${vibe.display_name}`}
          type="checkbox"
          checked={selected}
          onChange={(event) => onToggleSelected(event.target.checked)}
        />
        Select
      </label>
      <div className="grid grid-cols-3 gap-2">
        <AppButton variant="secondary" onClick={onEdit}>
          <Pencil aria-hidden="true" className="size-4" />
          Edit
        </AppButton>
        <AppButton variant="secondary" onClick={onExport} disabled={exportPending}>
          <Download aria-hidden="true" className="size-4" />
          Export
        </AppButton>
        <span className="flex items-center justify-center text-xs text-app-muted">
          {vibe.hidden ? "Hidden" : "Visible"}
        </span>
      </div>
      {onEnsureEncoding ? (
        <AppButton variant="secondary" onClick={onEnsureEncoding} disabled={encodePending}>
          <Sparkles aria-hidden="true" className="size-4" />
          Encode source
        </AppButton>
      ) : null}
      <div className="grid gap-1 text-xs text-app-muted">
        <span>{vibe.available_model_keys.length} models</span>
        <span>{vibe.available_encoding_configs.length} encoding configs</span>
        <span>{vibe.encodings.length} cached encodings</span>
        {vibe.hidden ? <span className="text-amber-200">{t("hidden")}</span> : null}
      </div>
    </article>
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
        <TextInput label="Local display name" value={name} onChange={setName} />
        <label className="flex h-9 items-center gap-2 border border-app-border bg-black/20 px-3 text-sm text-app-text">
          <input
            aria-label={t("hidden")}
            type="checkbox"
            checked={hidden}
            onChange={(event) => setHidden(event.target.checked)}
          />
          Hidden
        </label>
        <AppButton onClick={() => onSave(name, hidden)} disabled={saving || !name.trim()}>
          <Save aria-hidden="true" className="size-4" />
          Save changes
        </AppButton>
      </div>
    </AppModal>
  );
}
