/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { ClipboardPaste, FolderOpen, X } from "lucide-react";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { ResourceRefDto } from "@/types";

import { PreviewSlot } from "./ResourceEditorPrimitives";

export function ResourcePreviewEditor({
  resource,
  label,
  pending,
  error,
  onImport,
  onClear,
}: {
  resource: ResourceRefDto | null;
  label: string;
  pending: boolean;
  error: string | null;
  onImport: (source: "clipboard" | "file") => void;
  onClear: () => void;
}) {
  const { t } = useTranslation("resources");
  const handlePaste = useCallback(() => onImport("clipboard"), [onImport]);
  useEffect(() => {
    const listener = (event: globalThis.KeyboardEvent) => {
      const target = event.target;
      const editing =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        (target instanceof HTMLElement && target.isContentEditable);
      if (editing || event.key.toLowerCase() !== "v" || !(event.ctrlKey || event.metaKey)) return;
      event.preventDefault();
      handlePaste();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [handlePaste]);
  return (
    <div className="grid gap-3">
      <PreviewSlot resource={resource} label={label} />
      <p className="text-xs text-app-muted">{t("previewImportHint")}</p>
      {error ? <p className="text-xs text-rose-300">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        <AppButton variant="secondary" disabled={pending} onClick={() => onImport("file")}>
          <FolderOpen aria-hidden="true" className="size-4" />
          {t("choosePreviewFile")}
        </AppButton>
        <AppButton variant="secondary" disabled={pending} onClick={handlePaste}>
          <ClipboardPaste aria-hidden="true" className="size-4" />
          {t("pastePreview")}
        </AppButton>
        <AppButton variant="ghost" disabled={!resource || pending} onClick={onClear}>
          <X aria-hidden="true" className="size-4" />
          {t("clearPreview")}
        </AppButton>
      </div>
    </div>
  );
}
