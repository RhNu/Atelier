/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Download, Pencil, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { VibeDocumentEntryDto } from "@/types";

import type { ResourceViewMode } from "../resource-model";
import { PreviewSlot } from "./ResourceEditorPrimitives";

export function VibeCard({
  vibe,
  viewMode,
  selected,
  exportPending,
  encodePending,
  onToggleSelected,
  onExport,
  onEdit,
  onEnsureEncoding,
}: {
  vibe: VibeDocumentEntryDto;
  viewMode: ResourceViewMode;
  selected: boolean;
  exportPending: boolean;
  encodePending: boolean;
  onToggleSelected: (selected: boolean) => void;
  onExport: () => void;
  onEdit: () => void;
  onEnsureEncoding: (() => void) | null;
}) {
  const { t } = useTranslation("resources");
  const preview = (
    <PreviewSlot
      resource={vibe.preview ?? vibe.source_image}
      label={vibe.display_name}
      variant={viewMode}
    />
  );
  const details = (
    <div className="grid min-w-0 content-start gap-1 text-xs text-app-muted">
      <span className="truncate text-sm font-semibold text-app-text">{vibe.display_name}</span>
      <span>{t("modelCount", { count: vibe.available_model_keys.length })}</span>
      <span>{t("encodingConfigCount", { count: vibe.available_encoding_configs.length })}</span>
      <span>{t("cachedEncodingCount", { count: vibe.encodings.length })}</span>
      {vibe.hidden ? <span className="text-amber-200">{t("hidden")}</span> : null}
    </div>
  );
  const selection = (
    <label className="flex items-center gap-2 text-xs text-app-muted">
      <input
        aria-label={t("selectVibe", { name: vibe.display_name })}
        type="checkbox"
        checked={selected}
        onChange={(event) => onToggleSelected(event.target.checked)}
      />
      {t("select")}
    </label>
  );
  const actions = (
    <>
      <AppButton variant="secondary" onClick={onEdit}>
        <Pencil aria-hidden="true" className="size-4" />
        {t("edit")}
      </AppButton>
      <AppButton variant="secondary" onClick={onExport} disabled={exportPending}>
        <Download aria-hidden="true" className="size-4" />
        {t("export")}
      </AppButton>
      {onEnsureEncoding ? (
        <AppButton variant="secondary" onClick={onEnsureEncoding} disabled={encodePending}>
          <Sparkles aria-hidden="true" className="size-4" />
          {t("encodeSource")}
        </AppButton>
      ) : null}
    </>
  );
  if (viewMode === "list") {
    return (
      <article className="grid grid-cols-[44px_minmax(0,1fr)_auto] items-center gap-3 border border-app-border bg-app-surface px-2 py-1.5">
        {preview}
        <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-4">
          {details}
          {selection}
        </div>
        <div className="flex gap-1">{actions}</div>
      </article>
    );
  }
  return (
    <article className="grid gap-3 border border-app-border bg-app-surface p-3">
      {preview}
      {selection}
      <div className="grid grid-cols-2 gap-2">
        {actions}
        <span className="flex items-center justify-center text-xs text-app-muted">
          {vibe.hidden ? t("hidden") : t("visible")}
        </span>
      </div>
      {details}
    </article>
  );
}
