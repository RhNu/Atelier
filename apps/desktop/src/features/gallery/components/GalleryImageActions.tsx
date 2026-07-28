import {
  ChevronDown,
  Clapperboard,
  Copy,
  Download,
  LoaderCircle,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { ImageExportFormatDto } from "@/types";

const FORMAT_OPTIONS = [
  { value: "png_original", labelKey: "pngOriginal" },
  { value: "png_sanitized", labelKey: "pngSanitized" },
  { value: "jpeg", labelKey: "jpg" },
] as const;

type GalleryImageActionsProps = {
  onCopy: (format: ImageExportFormatDto) => void;
  onExport: (format: ImageExportFormatDto) => void;
  onSendToDirector: () => void;
  onDelete: () => void;
  copying: boolean;
  exporting: boolean;
  deleting: boolean;
  handoffPending: boolean;
};

function parseImageExportFormat(value: string | undefined): ImageExportFormatDto | null {
  switch (value) {
    case "png_original":
    case "png_sanitized":
    case "jpeg":
      return value;
    default:
      return null;
  }
}

function CompactActionMenu({
  label,
  icon: Icon,
  pending,
  onSelect,
}: {
  label: string;
  icon: LucideIcon;
  pending: boolean;
  onSelect: (format: ImageExportFormatDto) => void;
}) {
  const { t } = useTranslation("gallery");
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const toggleMenu = useCallback(() => setOpen((current) => !current), []);
  const selectOption = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const format = parseImageExportFormat(event.currentTarget.dataset.format);
      setOpen(false);
      if (format) {
        onSelect(format);
      }
    },
    [onSelect],
  );

  useEffect(() => {
    if (!open) {
      return;
    }
    const closeOutside = (event: PointerEvent) => {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) {
        setOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    window.addEventListener("pointerdown", closeOutside);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeOutside);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        className="inline-flex h-8 w-full items-center justify-center gap-1.5 border border-app-border bg-app-surface px-2 text-xs font-semibold text-app-text transition-colors hover:border-brand-400/60 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
        disabled={pending}
        onClick={toggleMenu}
      >
        {pending ? (
          <LoaderCircle aria-hidden="true" className="size-3.5 animate-spin" />
        ) : (
          <Icon aria-hidden="true" className="size-3.5" />
        )}
        {label}
        <ChevronDown aria-hidden="true" className="ml-auto size-3.5" />
      </button>
      {open ? (
        <div
          role="menu"
          aria-label={label}
          className="absolute top-full right-0 left-0 z-20 mt-1 border border-app-border bg-app-panel py-1 shadow-xl"
        >
          {FORMAT_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              role="menuitem"
              data-format={option.value}
              className="block w-full px-2.5 py-1.5 text-left text-xs text-app-text hover:bg-app-surface hover:text-white"
              onClick={selectOption}
            >
              {t(option.labelKey)}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function GalleryImageActions({
  onCopy,
  onExport,
  onSendToDirector,
  onDelete,
  copying,
  exporting,
  deleting,
  handoffPending,
}: GalleryImageActionsProps) {
  const { t } = useTranslation("gallery");

  return (
    <section className="grid gap-3">
      <div className="grid grid-cols-2 gap-2">
        <CompactActionMenu label={t("copyImage")} icon={Copy} pending={copying} onSelect={onCopy} />
        <CompactActionMenu
          label={t("exportImage")}
          icon={Download}
          pending={exporting}
          onSelect={onExport}
        />
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
