/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Plus, Save, Search, Trash2 } from "lucide-react";
import { Children, type ReactNode } from "react";

import { AppButton, AppPanel, EmptyState, ResourceImage } from "../../../components/ui";
import { resourceImageToDataUrl } from "../../../platform/atelier";
import type { CompiledPromptDto, ResourceRefDto } from "../../../types";
import { useResourceImageQuery } from "../data/useResourcesData";

export function SearchField({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex items-center gap-2 border border-app-border bg-app-panel px-3 py-1 text-sm text-app-muted">
      <Search aria-hidden="true" className="size-4" />
      <input
        aria-label="Search resources"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="Search resources"
        className="h-7 w-64 bg-transparent text-app-text outline-none placeholder:text-app-muted"
      />
    </label>
  );
}
export function ResourceEditorLayout({ list, editor }: { list: ReactNode; editor: ReactNode }) {
  return (
    <div className="grid min-h-0 flex-1 grid-cols-[360px_minmax(0,1fr)] gap-3">
      {list}
      {editor}
    </div>
  );
}

export function ResourceList({
  pending,
  error,
  emptyTitle,
  children,
}: {
  pending: boolean;
  error: string | null;
  emptyTitle: string;
  children: ReactNode;
}) {
  return (
    <AppPanel className="min-h-0 overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Library</h2>
      </header>
      <div className="min-h-0 overflow-auto p-3">
        {pending ? (
          <EmptyState title="Loading resources" />
        ) : error ? (
          <EmptyState title="Resources unavailable" description={error} />
        ) : Children.count(children) === 0 ? (
          <EmptyState title={emptyTitle} />
        ) : (
          <div className="grid gap-2">{children}</div>
        )}
      </div>
    </AppPanel>
  );
}

export function ResourceListButton({
  selected,
  title,
  detail,
  preview,
  onClick,
}: {
  selected: boolean;
  title: string;
  detail: string;
  preview: ResourceRefDto | null;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "grid grid-cols-[56px_minmax(0,1fr)] gap-3 border p-2 text-left",
        selected
          ? "border-brand-400/70 bg-brand-500/10"
          : "border-app-border bg-app-surface hover:border-brand-400/60",
      ].join(" ")}
    >
      <PreviewSlot resource={preview} label={title} compact />
      <span className="min-w-0">
        <span className="block truncate text-sm font-semibold text-app-text">{title}</span>
        <span className="mt-1 block truncate text-xs text-app-muted">{detail}</span>
      </span>
    </button>
  );
}

export function EditorPanel({
  title,
  subtitle,
  error,
  actions,
  children,
}: {
  title: string;
  subtitle: string;
  error: string | null;
  actions: ReactNode;
  children: ReactNode;
}) {
  return (
    <AppPanel className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <div>
          <h2 className="text-sm font-semibold text-white">{title}</h2>
          <p className="text-xs text-app-muted">{subtitle}</p>
        </div>
        {actions}
      </header>
      {error ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {error}
        </p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="grid gap-3">{children}</div>
      </div>
    </AppPanel>
  );
}

export function EditorActions({
  canDelete,
  saving,
  deleting,
  onNew,
  onSave,
  onDelete,
}: {
  canDelete: boolean;
  saving: boolean;
  deleting: boolean;
  onNew: () => void;
  onSave: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="flex gap-2">
      <AppButton variant="ghost" onClick={onNew}>
        <Plus aria-hidden="true" className="size-4" />
        New
      </AppButton>
      <AppButton variant="secondary" onClick={onSave} disabled={saving}>
        <Save aria-hidden="true" className="size-4" />
        Save
      </AppButton>
      <AppButton variant="danger" onClick={onDelete} disabled={!canDelete || deleting}>
        <Trash2 aria-hidden="true" className="size-4" />
        Delete
      </AppButton>
    </div>
  );
}

export function PreviewSlot({
  resource,
  label,
  compact = false,
}: {
  resource: ResourceRefDto | null;
  label: string;
  compact?: boolean;
}) {
  const imageQuery = useResourceImageQuery(resource);
  const src = imageQuery.data ? resourceImageToDataUrl(imageQuery.data) : null;
  return (
    <ResourceImage
      src={src}
      alt={label}
      fallbackLabel={compact ? "" : "No preview"}
      className={
        compact
          ? "size-14 border border-app-border"
          : "aspect-video w-full border border-app-border"
      }
    />
  );
}

export function CompiledPreview({ preview }: { preview: CompiledPromptDto | null }) {
  if (!preview) {
    return null;
  }
  return (
    <article className="border border-app-border bg-black/20 p-3">
      <p className="text-xs font-semibold text-app-muted uppercase">Compiled preview</p>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {preview.expanded_prompt || "Empty"}
      </p>
      <p className="mt-2 text-xs text-app-muted">
        {preview.trace.function_calls.length} function calls
      </p>
    </article>
  );
}

export function TextInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

export function NumberInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        type="number"
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || 0)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

export function TextArea({
  label,
  value,
  minRows,
  onChange,
}: {
  label: string;
  value: string;
  minRows: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <textarea
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={[
          minRows,
          "resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400",
        ].join(" ")}
      />
    </label>
  );
}

export function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex h-9 items-center gap-2 border border-app-border bg-black/20 px-3 text-sm text-app-text">
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  );
}
