/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Search } from "lucide-react";
import { Children, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel, EmptyState, ResourceImage } from "@/components/ui";
import { resourceImageToDataUrl } from "@/platform/atelier";
import type { ResourceRefDto } from "@/types";

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
