/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { ResourcePickerDialog } from "@/features/resources";
import type { ImageModelDto, VibeDocumentEntryDto } from "@/types";

import { useVibeDocumentsQuery } from "../data/useGenerationActions";
import { findVibeEncodingForModel } from "./vibe-guidance-model";

export function VibeLibraryDialog({
  open,
  model,
  onClose,
  onSelect,
}: {
  open: boolean;
  model: ImageModelDto;
  onClose: () => void;
  onSelect: (entry: VibeDocumentEntryDto) => void;
}) {
  const { t } = useTranslation("generation");
  const query = useVibeDocumentsQuery(
    { offset: 0, limit: 200, include_hidden: false, model },
    open,
  );
  const entries = useMemo(
    () =>
      (query.data?.items ?? []).filter((entry) => findVibeEncodingForModel(entry, model) !== null),
    [model, query.data?.items],
  );
  const pickerItems = useMemo(
    () =>
      entries.map((entry) => ({
        id: entry.vibe_id,
        label: entry.display_name,
        preview: entry.preview ?? entry.source_image,
        entry,
      })),
    [entries],
  );

  return (
    <ResourcePickerDialog
      open={open}
      title={t("vibeLibrary")}
      namespace="vibe"
      items={pickerItems}
      pending={query.isPending}
      error={query.error}
      emptyLabel={t("noCompatibleVibes")}
      noMatchesLabel={t("noCompatibleVibes")}
      onClose={onClose}
      onSelect={(item) => onSelect(item.entry)}
    />
  );
}
