import { Clapperboard, Download, Maximize2, ShieldCheck, Trash2 } from "lucide-react";
import type { ChangeEvent } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppPanel, AppSelect, EmptyState, SafetyBadge } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { GalleryItemDto } from "@/types";

import {
  effectiveSafetyLabel,
  formatScore,
  formatTimestamp,
  overrideOptions,
  preferredExportAsset,
  preferredPreviewResource,
} from "../gallery-utils";
import { GalleryItemImage } from "./GalleryItemImage";

type GalleryInspectorProps = {
  item: GalleryItemDto | null;
  blurSensitive: boolean;
  overrideValue: string;
  onOverrideChange: (value: string) => void;
  onApplyOverride: () => void;
  onExport: () => void;
  onSendToDirector: () => void;
  onDelete: () => void;
  applyingOverride: boolean;
  exporting: boolean;
  deleting: boolean;
  handoffPending: boolean;
  commandError: string | null;
};

function DetailRow({ label, value }: { label: string; value: string | number | null }) {
  if (value === null || value === "") {
    return null;
  }

  return (
    <div className="grid grid-cols-[96px_minmax(0,1fr)] gap-3 text-sm">
      <dt className="text-app-muted">{label}</dt>
      <dd className="min-w-0 break-words text-app-text">{value}</dd>
    </div>
  );
}

function EmptyInspector() {
  const { t } = useTranslation("gallery");
  return (
    <AppPanel
      as="aside"
      variant="section"
      aria-label={t("detailsLabel")}
      className="min-h-0 overflow-auto"
    >
      <div className="p-3">
        <EmptyState title={t("noSelection")} description={t("selectToInspect")} />
      </div>
    </AppPanel>
  );
}

function ArtifactDetails({ item }: { item: GalleryItemDto }) {
  const { t } = useTranslation("gallery");
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">{t("artifact")}</h3>
      <dl className="grid gap-2">
        <DetailRow label={t("artifact")} value={item.artifact_id} />
        <DetailRow label={t("kind")} value={item.artifact_kind} />
        <DetailRow label={t("source")} value={item.source_kind} />
        <DetailRow label={t("model")} value={item.model_name} />
        <DetailRow
          label={t("seed", { value: "" })}
          value={item.seed === null ? null : t("seed", { value: item.seed })}
        />
        <DetailRow
          label={t("sample", { value: "" })}
          value={item.sample_index === null ? null : t("sample", { value: item.sample_index })}
        />
        <DetailRow label={t("indexed")} value={formatTimestamp(item.indexed_at_ms)} />
      </dl>
    </section>
  );
}

function AssetDetails({ item }: { item: GalleryItemDto }) {
  const { t } = useTranslation("gallery");
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">{t("assets")}</h3>
      <div className="grid gap-2">
        {item.assets.map((asset) => (
          <div
            key={`${asset.role}-${asset.resource.id}-${asset.resource.variant_id ?? "base"}`}
            className={[
              "grid grid-cols-[72px_minmax(0,1fr)] gap-3 border border-app-border",
              "bg-app-surface px-3 py-2 text-sm",
            ].join(" ")}
          >
            <span className="font-semibold text-app-text">{asset.role}</span>
            <span className="truncate text-app-muted">{asset.resource.id}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

function SafetyDetails({ item }: { item: GalleryItemDto }) {
  const { t } = useTranslation("gallery");
  const nsfwScore = formatScore("NSFW", item.safety?.nsfw_score ?? null);
  const safeScore = formatScore("Safe", item.safety?.safe_score ?? null);

  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">{t("safety")}</h3>
      <dl className="grid gap-2">
        <DetailRow label={t("label")} value={effectiveSafetyLabel(item)} />
        <DetailRow label="NSFW" value={nsfwScore} />
        <DetailRow label={t("safe")} value={safeScore} />
        <DetailRow label={t("model")} value={item.safety?.model_id ?? null} />
        <DetailRow label={t("version")} value={item.safety?.scorer_version ?? null} />
      </dl>
      {item.safety?.raw_scores.length ? (
        <div className="grid gap-1 border border-app-border bg-app-surface p-3">
          {item.safety.raw_scores.map((score) => (
            <p
              key={`${score.label}-${score.score}`}
              className="flex justify-between gap-3 text-xs text-app-muted"
            >
              <span>{score.label}</span>
              <span>{score.score.toFixed(2)}</span>
            </p>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function InspectorActions({
  item,
  overrideValue,
  onOverrideChange,
  onApplyOverride,
  onExport,
  onSendToDirector,
  onDelete,
  applyingOverride,
  exporting,
  deleting,
  handoffPending,
}: GalleryInspectorProps & { item: GalleryItemDto }) {
  const { t } = useTranslation("gallery");
  const localizedOverrides = useMemo(
    () => overrideOptions.map((option) => ({ ...option, label: t(option.labelKey) })),
    [t],
  );
  const handleOverrideChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => onOverrideChange(event.target.value),
    [onOverrideChange],
  );
  const exportAsset = preferredExportAsset(item);

  return (
    <section className="grid gap-3">
      <label
        htmlFor="gallery-safety-override"
        className="grid gap-1 text-sm font-semibold text-app-text"
      >
        {t("safetyOverride")}
        <AppSelect
          id="gallery-safety-override"
          aria-label={t("safetyOverride")}
          options={localizedOverrides}
          value={overrideValue}
          onChange={handleOverrideChange}
        />
      </label>
      <div className="grid grid-cols-2 gap-2">
        <AppButton
          variant="secondary"
          onClick={onApplyOverride}
          disabled={applyingOverride}
          className="w-full"
        >
          <ShieldCheck aria-hidden="true" className="size-4" />
          {t("applyOverride")}
        </AppButton>
        <AppButton variant="secondary" onClick={onExport} disabled={exporting} className="w-full">
          <Download aria-hidden="true" className="size-4" />
          {t("exportImage")}
        </AppButton>
      </div>
      <AppButton
        variant="secondary"
        onClick={onSendToDirector}
        disabled={handoffPending}
        className="w-full"
      >
        <Clapperboard aria-hidden="true" className="size-4" />
        {t("sendDirector")}
      </AppButton>
      <AppButton variant="danger" onClick={onDelete} disabled={deleting} className="w-full">
        <Trash2 aria-hidden="true" className="size-4" />
        {t("deleteSelected")}
      </AppButton>
      <p className="text-xs text-app-muted">{t("exportTarget", { role: exportAsset.role })}</p>
    </section>
  );
}

export function GalleryInspector(props: GalleryInspectorProps) {
  const { t } = useTranslation("gallery");
  const { item, blurSensitive } = props;
  const pushToast = useToastStore((state) => state.push);
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const openLightbox = useCallback(() => setLightboxOpen(true), []);
  const closeLightbox = useCallback(() => setLightboxOpen(false), []);
  useEffect(() => {
    if (props.commandError) {
      pushToast({ level: "error", title: t("actionFailed"), message: props.commandError });
    }
  }, [props.commandError, pushToast, t]);

  if (!item) {
    return <EmptyInspector />;
  }

  return (
    <AppPanel
      as="aside"
      variant="section"
      aria-label={t("detailsLabel")}
      className="min-h-0 overflow-auto"
    >
      <header className="border-b border-app-border px-4 py-3">
        <div className="flex min-w-0 items-center justify-between gap-3">
          <div className="min-w-0">
            <p className="text-xs font-semibold text-brand-200 uppercase">{t("details")}</p>
            <h2 className="truncate text-base font-semibold text-white">{item.item_id}</h2>
          </div>
          <SafetyBadge label={effectiveSafetyLabel(item)} />
        </div>
      </header>
      <div className="grid gap-4 p-4">
        <button
          type="button"
          aria-label={t("enlargeItem", { id: item.item_id })}
          className="cursor-zoom-in"
          onClick={openLightbox}
        >
          <GalleryItemImage
            item={item}
            resource={preferredPreviewResource(item)}
            alt={t("detailPreview", { id: item.item_id })}
            className="aspect-square w-full border border-app-border bg-app-bg"
            blurSensitive={blurSensitive}
          />
        </button>
        <AppButton variant="secondary" onClick={openLightbox}>
          <Maximize2 aria-hidden="true" className="size-4" />
          {t("enlargeImage")}
        </AppButton>
        <ArtifactDetails item={item} />
        <AssetDetails item={item} />
        <SafetyDetails item={item} />
        <InspectorActions {...props} item={item} />
      </div>
      <AppModal open={lightboxOpen} title={item.item_id} size="fullscreen" onClose={closeLightbox}>
        <div className="flex h-full min-h-0 items-center justify-center bg-black/40">
          <GalleryItemImage
            item={item}
            resource={preferredPreviewResource(item)}
            alt={t("enlargedPreview", { id: item.item_id })}
            className="max-h-full max-w-full object-contain"
            blurSensitive={blurSensitive}
          />
        </div>
      </AppModal>
    </AppPanel>
  );
}
