/* eslint-disable max-lines, react-perf/jsx-no-new-function-as-prop */
import { Plus, Save, Search, Trash2 } from "lucide-react";
import { Children, type ChangeEvent, type ReactNode, useId } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, AppSelect, EmptyState, ResourceImage } from "@/components/ui";
import { NaiPromptEditor } from "@/features/prompt-editor";
import { resourceImageToDataUrl } from "@/platform/atelier";
import type { CompiledPromptDto, ResourceRefDto } from "@/types";

import { useResourceImageQuery } from "../data/useResourcesData";
import type { ResourceViewMode } from "../resource-model";

export function SearchField({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation("resources");
  return (
    <label className="flex items-center gap-2 border border-app-border bg-app-panel px-3 py-1 text-sm text-app-muted">
      <Search aria-hidden="true" className="size-4" />
      <input
        aria-label={t("search")}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t("search")}
        className="h-7 w-64 bg-transparent text-app-text outline-none placeholder:text-app-muted"
      />
    </label>
  );
}
export function ResourceList({
  pending,
  error,
  emptyTitle,
  actions,
  viewMode,
  children,
}: {
  pending: boolean;
  error: string | null;
  emptyTitle: string;
  actions?: ReactNode;
  viewMode: ResourceViewMode;
  children: ReactNode;
}) {
  const { t } = useTranslation("resources");
  return (
    <AppPanel variant="section" className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">{t("library")}</h2>
        {actions}
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {pending ? (
          <EmptyState title={t("loading")} />
        ) : error ? (
          <EmptyState title={t("unavailable")} description={error} />
        ) : Children.count(children) === 0 ? (
          <EmptyState title={emptyTitle} iconOnly />
        ) : (
          <div
            className={
              viewMode === "grid"
                ? "grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3"
                : "grid gap-1"
            }
          >
            {children}
          </div>
        )}
      </div>
    </AppPanel>
  );
}

export function ResourceListButton({
  selected,
  title,
  detail,
  description,
  preview,
  viewMode,
  onClick,
}: {
  selected: boolean;
  title: string;
  detail: string;
  description?: string | null;
  preview: ResourceRefDto | null;
  viewMode: ResourceViewMode;
  onClick: () => void;
}) {
  const selectedClass = selected
    ? "border-brand-400/70 bg-brand-500/10"
    : "border-app-border bg-app-surface hover:border-brand-400/60";
  if (viewMode === "grid") {
    return (
      <button
        type="button"
        onClick={onClick}
        className={["group grid content-start border text-left", selectedClass].join(" ")}
      >
        <PreviewSlot resource={preview} label={title} variant="grid" />
        <span className="min-w-0 border-t border-app-border px-3 py-2.5">
          <span className="block truncate text-sm font-semibold text-app-text">{title}</span>
          <span className="mt-1 block truncate text-xs text-app-muted">{detail}</span>
        </span>
      </button>
    );
  }
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "grid grid-cols-[44px_minmax(0,1fr)] items-center gap-3 border px-2 py-1.5 text-left",
        selectedClass,
      ].join(" ")}
    >
      <PreviewSlot resource={preview} label={title} variant="list" />
      <span className="min-w-0">
        <span className="block truncate text-sm font-semibold text-app-text">{title}</span>
        <span className="mt-0.5 block truncate text-xs text-app-muted">{detail}</span>
        {description ? (
          <span className="mt-0.5 block truncate text-[11px] text-app-muted/80">{description}</span>
        ) : null}
      </span>
    </button>
  );
}

export function EditorPanel({
  error,
  actions,
  children,
}: {
  error: string | null;
  actions: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-3">
      {error ? (
        <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {error}
        </p>
      ) : null}
      <div className="grid gap-3">{children}</div>
      <footer className="flex items-center justify-end border-t border-app-border pt-3">
        {actions}
      </footer>
    </div>
  );
}

export function EditorActions({
  canDelete,
  saving,
  deleting,
  onSave,
  onDelete,
}: {
  canDelete: boolean;
  saving: boolean;
  deleting: boolean;
  onSave: () => void;
  onDelete: () => void;
}) {
  const { t: translateCommon } = useTranslation("common");
  return (
    <div className="flex w-full items-center justify-between gap-2">
      <span>
        {canDelete ? (
          <AppButton variant="danger" onClick={onDelete} disabled={deleting || saving}>
            <Trash2 aria-hidden="true" className="size-4" />
            {translateCommon("delete")}
          </AppButton>
        ) : null}
      </span>
      <AppButton onClick={onSave} disabled={saving || deleting}>
        {canDelete ? (
          <Save aria-hidden="true" className="size-4" />
        ) : (
          <Plus aria-hidden="true" className="size-4" />
        )}
        {saving ? translateCommon("saving") : translateCommon(canDelete ? "save" : "create")}
      </AppButton>
    </div>
  );
}

export function PreviewSlot({
  resource,
  label,
  variant = "editor",
}: {
  resource: ResourceRefDto | null;
  label: string;
  variant?: "editor" | "list" | "grid";
}) {
  const { t } = useTranslation("resources");
  const imageQuery = useResourceImageQuery(resource);
  const src = imageQuery.data ? resourceImageToDataUrl(imageQuery.data) : null;
  return (
    <ResourceImage
      src={src}
      alt={label}
      fallbackLabel={variant === "list" ? "" : t("noPreview")}
      className={
        {
          editor: "aspect-video w-full border border-app-border",
          grid: "aspect-square w-full border-app-border bg-black/20",
          list: "size-11 border border-app-border opacity-60",
        }[variant]
      }
    />
  );
}

export function CompiledPreview({ preview }: { preview: CompiledPromptDto | null }) {
  const { t } = useTranslation("resources");
  if (!preview) {
    return null;
  }
  return (
    <article className="border border-app-border bg-black/20 p-3">
      <p className="text-xs font-semibold text-app-muted uppercase">{t("compiledPreview")}</p>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {preview.expanded_prompt || t("empty")}
      </p>
      <p className="mt-2 text-xs text-app-muted">
        {t("functionCalls", { count: preview.trace.function_calls.length })}
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

export function CategoryInput({
  label,
  value,
  suggestions,
  onChange,
}: {
  label: string;
  value: string;
  suggestions: ReadonlyArray<string>;
  onChange: (value: string) => void;
}) {
  const suggestionsId = useId();
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        list={suggestions.length > 0 ? suggestionsId : undefined}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
      {suggestions.length > 0 ? (
        <datalist id={suggestionsId}>
          {suggestions.map((suggestion) => (
            <option key={suggestion} value={suggestion}>
              {suggestion}
            </option>
          ))}
        </datalist>
      ) : null}
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

export function PromptTextArea({
  label,
  value,
  minHeight = 160,
  onChange,
}: {
  label: string;
  value: string;
  minHeight?: number;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted">
      {label}
      <NaiPromptEditor
        aria-label={label}
        value={value}
        onChange={onChange}
        profile="novelai_v45"
        minHeight={minHeight}
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

export function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: ReadonlyArray<{ value: string; label: string }>;
  onChange: (value: string) => void;
}) {
  const handleChange = (event: ChangeEvent<HTMLSelectElement>) => onChange(event.target.value);
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <AppSelect aria-label={label} value={value} options={options} onChange={handleChange} />
    </label>
  );
}
