import { Check, PencilLine, Power, RefreshCw, Trash2, X } from "lucide-react";
import { useCallback } from "react";

import { AppButton } from "../../../components/ui";
import type { ApiKeyRecordDto, SubscriptionSummaryDto } from "../../../types";
import { TextField } from "./SettingsControls";

export type ApiKeyEditState = {
  id: string;
  displayName: string;
  secret: string;
};

export type ApiKeyProbeState = {
  subscription: SubscriptionSummaryDto | null;
  error: string | null;
};

export function ApiKeyRow({
  item,
  editing,
  probeState,
  busy,
  onEdit,
  onCancelEdit,
  onEditChange,
  onSaveEdit,
  onSetActive,
  onProbe,
  onDelete,
}: {
  item: ApiKeyRecordDto;
  editing: ApiKeyEditState | null;
  probeState: ApiKeyProbeState | undefined;
  busy: boolean;
  onEdit: (editing: ApiKeyEditState) => void;
  onCancelEdit: () => void;
  onEditChange: (editing: ApiKeyEditState) => void;
  onSaveEdit: () => void;
  onSetActive: (id: string) => void;
  onProbe: (item: ApiKeyRecordDto) => void;
  onDelete: (id: string) => void;
}) {
  const isEditing = editing?.id === item.id;
  const startEdit = useCallback(() => {
    onEdit({ id: item.id, displayName: item.display_name, secret: "" });
  }, [item.display_name, item.id, onEdit]);
  const setActive = useCallback(() => {
    onSetActive(item.id);
  }, [item.id, onSetActive]);
  const probe = useCallback(() => {
    onProbe(item);
  }, [item, onProbe]);
  const deleteKey = useCallback(() => {
    onDelete(item.id);
  }, [item.id, onDelete]);

  return (
    <article className="border-b border-app-border p-3 last:border-b-0">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <ApiKeyIdentity item={item} probeState={probeState} />
        <ApiKeyActions
          item={item}
          busy={busy}
          onSetActive={setActive}
          onProbe={probe}
          onEdit={startEdit}
          onDelete={deleteKey}
        />
      </div>
      {isEditing && editing ? (
        <ApiKeyEditForm
          editing={editing}
          busy={busy}
          onEditChange={onEditChange}
          onSaveEdit={onSaveEdit}
          onCancelEdit={onCancelEdit}
        />
      ) : null}
    </article>
  );
}

function ApiKeyIdentity({
  item,
  probeState,
}: {
  item: ApiKeyRecordDto;
  probeState: ApiKeyProbeState | undefined;
}) {
  return (
    <div className="min-w-0">
      <div className="flex flex-wrap items-center gap-2">
        <p className="truncate text-sm font-semibold text-app-text">{item.display_name}</p>
        {item.is_active ? (
          <span className="border border-emerald-500/45 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-200 uppercase">
            Active
          </span>
        ) : null}
      </div>
      <p className="mt-1 text-xs break-all text-app-muted">ID {item.id}</p>
      {probeState?.subscription ? (
        <p className="mt-2 text-xs text-app-muted">
          Probe {probeState.subscription.tier_name} / {probeState.subscription.anlas_balance} Anlas
        </p>
      ) : null}
      {probeState?.error ? <p className="mt-2 text-xs text-amber-200">{probeState.error}</p> : null}
    </div>
  );
}

function ApiKeyActions({
  item,
  busy,
  onSetActive,
  onProbe,
  onEdit,
  onDelete,
}: {
  item: ApiKeyRecordDto;
  busy: boolean;
  onSetActive: () => void;
  onProbe: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {!item.is_active ? (
        <AppButton
          variant="ghost"
          className="h-8 px-2 text-xs"
          disabled={busy}
          aria-label={`Set ${item.display_name} active`}
          onClick={onSetActive}
        >
          <Power aria-hidden="true" className="size-3.5" />
          Activate
        </AppButton>
      ) : null}
      <AppButton
        variant="ghost"
        className="h-8 px-2 text-xs"
        disabled={busy}
        aria-label={`Probe ${item.display_name}`}
        onClick={onProbe}
      >
        <RefreshCw aria-hidden="true" className="size-3.5" />
        Probe
      </AppButton>
      <AppButton
        variant="ghost"
        className="h-8 px-2 text-xs"
        disabled={busy}
        aria-label={`Edit ${item.display_name}`}
        onClick={onEdit}
      >
        <PencilLine aria-hidden="true" className="size-3.5" />
        Edit
      </AppButton>
      <AppButton
        variant="danger"
        className="h-8 px-2 text-xs"
        disabled={busy}
        aria-label={`Delete ${item.display_name}`}
        onClick={onDelete}
      >
        <Trash2 aria-hidden="true" className="size-3.5" />
        Delete
      </AppButton>
    </div>
  );
}

function ApiKeyEditForm({
  editing,
  busy,
  onEditChange,
  onSaveEdit,
  onCancelEdit,
}: {
  editing: ApiKeyEditState;
  busy: boolean;
  onEditChange: (editing: ApiKeyEditState) => void;
  onSaveEdit: () => void;
  onCancelEdit: () => void;
}) {
  const updateDisplayName = useCallback(
    (displayName: string) => {
      onEditChange({ ...editing, displayName });
    },
    [editing, onEditChange],
  );
  const updateSecret = useCallback(
    (secret: string) => {
      onEditChange({ ...editing, secret });
    },
    [editing, onEditChange],
  );

  return (
    <div className="mt-3 grid gap-2 border-t border-app-border pt-3 md:grid-cols-[1fr_1.3fr_auto]">
      <TextField
        label="Edit API key display name"
        value={editing.displayName}
        onChange={updateDisplayName}
      />
      <TextField
        label="Replace API key secret"
        value={editing.secret}
        onChange={updateSecret}
        type="password"
        autoComplete="new-password"
      />
      <div className="flex items-end gap-2">
        <AppButton disabled={busy} aria-label="Save API key changes" onClick={onSaveEdit}>
          <Check aria-hidden="true" className="size-4" />
          Save
        </AppButton>
        <AppButton variant="ghost" disabled={busy} onClick={onCancelEdit}>
          <X aria-hidden="true" className="size-4" />
          Cancel
        </AppButton>
      </div>
    </div>
  );
}
