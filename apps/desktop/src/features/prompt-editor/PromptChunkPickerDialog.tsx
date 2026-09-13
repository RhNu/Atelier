/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { ResourcePickerDialog } from "@/features/resources/components/ResourcePickerDialog";
import type { ImageModelDto } from "@/types";

import { usePromptChunkPickerQuery } from "./data/usePromptChunkPickerQuery";

export function PromptChunkPickerDialog({
  open,
  model,
  onClose,
  onSelect,
}: {
  open: boolean;
  model: ImageModelDto | null;
  onClose: () => void;
  onSelect: (path: string) => void;
}) {
  const { t } = useTranslation("promptEditor");
  const { t: resourceT } = useTranslation("resources");
  const query = usePromptChunkPickerQuery(model, open);
  const items = useMemo(
    () =>
      (query.data?.items ?? []).map((chunk) => ({
        id: chunk.chunk_id,
        label: chunk.display_name || chunk.identifier,
        aliases: chunk.aliases,
        description: chunk.description ?? chunk.content,
        preview: chunk.preview,
        path: chunk.path,
      })),
    [query.data?.items],
  );

  return (
    <ResourcePickerDialog
      open={open}
      title={t("chunkLibrary")}
      namespace="prompt_chunk"
      items={items}
      pending={query.isPending}
      error={query.error}
      emptyLabel={resourceT("noPromptChunks")}
      noMatchesLabel={t("noMatchingChunks")}
      onClose={onClose}
      onSelect={(item) => onSelect(item.path)}
    />
  );
}
