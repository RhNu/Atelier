import { Clapperboard, Download, Maximize2, ShieldCheck, Trash2 } from "lucide-react";
import type { ChangeEvent } from "react";
import { useCallback, useState } from "react";

import {
  AppButton,
  AppModal,
  AppPanel,
  AppSelect,
  EmptyState,
  SafetyBadge,
} from "../../../components/ui";
import type { GalleryItemDto } from "../../../types";
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
  return (
    <AppPanel
      as="aside"
      variant="section"
      aria-label="Gallery item details"
      className="min-h-0 overflow-auto"
    >
      <div className="p-3">
        <EmptyState title="No gallery item selected" description="Select an image to inspect it." />
      </div>
    </AppPanel>
  );
}

function ArtifactDetails({ item }: { item: GalleryItemDto }) {
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">Artifact</h3>
      <dl className="grid gap-2">
        <DetailRow label="Artifact" value={item.artifact_id} />
        <DetailRow label="Kind" value={item.artifact_kind} />
        <DetailRow label="Source" value={item.source_kind} />
        <DetailRow label="Model" value={item.model_name} />
        <DetailRow label="Seed" value={item.seed === null ? null : `Seed ${item.seed}`} />
        <DetailRow
          label="Sample"
          value={item.sample_index === null ? null : `Sample ${item.sample_index}`}
        />
        <DetailRow label="Indexed" value={formatTimestamp(item.indexed_at_ms)} />
      </dl>
    </section>
  );
}

function AssetDetails({ item }: { item: GalleryItemDto }) {
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">Assets</h3>
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
  const nsfwScore = formatScore("NSFW", item.safety?.nsfw_score ?? null);
  const safeScore = formatScore("Safe", item.safety?.safe_score ?? null);

  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-white">Safety</h3>
      <dl className="grid gap-2">
        <DetailRow label="Label" value={effectiveSafetyLabel(item)} />
        <DetailRow label="NSFW" value={nsfwScore} />
        <DetailRow label="Safe" value={safeScore} />
        <DetailRow label="Model" value={item.safety?.model_id ?? null} />
        <DetailRow label="Version" value={item.safety?.scorer_version ?? null} />
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
        Safety override
        <AppSelect
          id="gallery-safety-override"
          aria-label="Safety override"
          options={overrideOptions}
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
          Apply safety override
        </AppButton>
        <AppButton variant="secondary" onClick={onExport} disabled={exporting} className="w-full">
          <Download aria-hidden="true" className="size-4" />
          Export selected image
        </AppButton>
      </div>
      <AppButton
        variant="secondary"
        onClick={onSendToDirector}
        disabled={handoffPending}
        className="w-full"
      >
        <Clapperboard aria-hidden="true" className="size-4" />
        Send to Director
      </AppButton>
      <AppButton variant="danger" onClick={onDelete} disabled={deleting} className="w-full">
        <Trash2 aria-hidden="true" className="size-4" />
        Delete selected gallery item
      </AppButton>
      <p className="text-xs text-app-muted">Export target: {exportAsset.role}</p>
    </section>
  );
}

export function GalleryInspector(props: GalleryInspectorProps) {
  const { item, blurSensitive } = props;
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const openLightbox = useCallback(() => setLightboxOpen(true), []);
  const closeLightbox = useCallback(() => setLightboxOpen(false), []);

  if (!item) {
    return <EmptyInspector />;
  }

  return (
    <AppPanel
      as="aside"
      variant="section"
      aria-label="Gallery item details"
      className="min-h-0 overflow-auto"
    >
      <header className="border-b border-app-border px-4 py-3">
        <div className="flex min-w-0 items-center justify-between gap-3">
          <div className="min-w-0">
            <p className="text-xs font-semibold text-brand-200 uppercase">Details</p>
            <h2 className="truncate text-base font-semibold text-white">{item.item_id}</h2>
          </div>
          <SafetyBadge label={effectiveSafetyLabel(item)} />
        </div>
      </header>
      <div className="grid gap-4 p-4">
        {props.commandError ? (
          <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
            {props.commandError}
          </p>
        ) : null}
        <button
          type="button"
          aria-label={`Enlarge ${item.item_id}`}
          className="cursor-zoom-in"
          onClick={openLightbox}
        >
          <GalleryItemImage
            item={item}
            resource={preferredPreviewResource(item)}
            alt={`${item.item_id} detail preview`}
            className="aspect-square w-full border border-app-border bg-app-bg"
            blurSensitive={blurSensitive}
          />
        </button>
        <AppButton variant="secondary" onClick={openLightbox}>
          <Maximize2 aria-hidden="true" className="size-4" />
          Enlarge image
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
            alt={`${item.item_id} enlarged preview`}
            className="max-h-full max-w-full object-contain"
            blurSensitive={blurSensitive}
          />
        </div>
      </AppModal>
    </AppPanel>
  );
}
