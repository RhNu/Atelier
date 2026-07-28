import { Clapperboard, Download, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppPanel, EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { GalleryItemDto } from "@/types";

import {
  displayGalleryArtifactKind,
  displayGalleryModelName,
  displayGallerySource,
  formatTimestamp,
  preferredPreviewResource,
} from "../gallery-utils";
import { GalleryItemImage } from "./GalleryItemImage";
import { GallerySafetyDetails } from "./GallerySafetyDetails";

type GalleryInspectorProps = {
  item: GalleryItemDto | null;
  items: GalleryItemDto[];
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
  onSelectItem: (itemId: string) => void;
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
        <DetailRow label={t("kind")} value={displayGalleryArtifactKind(item.artifact_kind, t)} />
        <DetailRow label={t("source")} value={displayGallerySource(item.source_kind, t)} />
        <DetailRow label={t("model")} value={displayGalleryModelName(item.model_name)} />
        <DetailRow label={t("seedLabel")} value={item.seed} />
        <DetailRow label={t("sampleLabel")} value={item.sample_index} />
        <DetailRow label={t("indexed")} value={formatTimestamp(item.indexed_at_ms)} />
      </dl>
    </section>
  );
}

function AssetDetails({ item }: { item: GalleryItemDto }) {
  const { t } = useTranslation("gallery");
  return (
    <details className="border-t border-app-border pt-3">
      <summary className="cursor-pointer text-sm font-semibold text-white">{t("assets")}</summary>
      <dl className="mt-2 grid gap-1.5 text-xs">
        {item.assets.map((asset) => (
          <div
            key={`${asset.role}-${asset.resource.id}-${asset.resource.variant_id ?? "base"}`}
            className="grid grid-cols-[72px_minmax(0,1fr)] gap-3"
          >
            <dt className="font-semibold text-app-text">{asset.role}</dt>
            <dd className="break-all text-app-muted">
              {asset.resource.id}
              {asset.resource.variant_id ? ` (${asset.resource.variant_id})` : ""}
            </dd>
          </div>
        ))}
      </dl>
    </details>
  );
}

function InspectorActions({
  onExport,
  onSendToDirector,
  onDelete,
  exporting,
  deleting,
  handoffPending,
}: Pick<
  GalleryInspectorProps,
  "onExport" | "onSendToDirector" | "onDelete" | "exporting" | "deleting" | "handoffPending"
>) {
  const { t } = useTranslation("gallery");

  return (
    <section className="grid gap-3">
      <div className="grid grid-cols-2 gap-2">
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
    </section>
  );
}

export function GalleryInspector(props: GalleryInspectorProps) {
  const { t } = useTranslation("gallery");
  const { item, items, blurSensitive, onSelectItem } = props;
  const pushToast = useToastStore((state) => state.push);
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const openLightbox = useCallback(() => setLightboxOpen(true), []);
  const closeLightbox = useCallback(() => setLightboxOpen(false), []);
  const selectedIndex = item
    ? items.findIndex((candidate) => candidate.item_id === item.item_id)
    : -1;
  const selectPreviousItem = useCallback(() => {
    if (selectedIndex > 0) {
      onSelectItem(items[selectedIndex - 1].item_id);
    }
  }, [items, onSelectItem, selectedIndex]);
  const selectNextItem = useCallback(() => {
    if (selectedIndex >= 0 && selectedIndex < items.length - 1) {
      onSelectItem(items[selectedIndex + 1].item_id);
    }
  }, [items, onSelectItem, selectedIndex]);

  useEffect(() => {
    if (!lightboxOpen) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        selectPreviousItem();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        selectNextItem();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [lightboxOpen, selectNextItem, selectPreviousItem]);

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
            <h2 className="text-base font-semibold text-white">{t("details")}</h2>
          </div>
        </div>
      </header>
      <div className="grid gap-4 p-4">
        <button
          type="button"
          aria-label={t("enlargeImage")}
          className="cursor-zoom-in outline-none focus-visible:ring-2 focus-visible:ring-brand-400 focus-visible:ring-offset-2 focus-visible:ring-offset-app-panel"
          onClick={openLightbox}
        >
          <GalleryItemImage
            item={item}
            resource={preferredPreviewResource(item)}
            alt={t("galleryDetailImageAlt")}
            className="aspect-square w-full border border-app-border bg-app-bg"
            blurSensitive={blurSensitive}
          />
        </button>
        <ArtifactDetails item={item} />
        <GallerySafetyDetails
          item={item}
          overrideValue={props.overrideValue}
          onOverrideChange={props.onOverrideChange}
          onApplyOverride={props.onApplyOverride}
          applyingOverride={props.applyingOverride}
        />
        <AssetDetails item={item} />
        <InspectorActions {...props} />
      </div>
      <AppModal
        open={lightboxOpen}
        title={t("imagePreview")}
        size="fullscreen"
        hideHeader
        onClose={closeLightbox}
      >
        <div className="flex h-full min-h-0 items-center justify-center bg-black/40 p-4">
          <GalleryItemImage
            item={item}
            resource={preferredPreviewResource(item)}
            alt={t("galleryEnlargedImageAlt")}
            className="max-h-full max-w-full object-contain"
            blurSensitive={blurSensitive}
          />
        </div>
      </AppModal>
    </AppPanel>
  );
}
