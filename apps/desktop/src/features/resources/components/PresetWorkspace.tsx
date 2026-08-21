/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { ImageModelDto, PromptPresetDto, PromptPresetKindDto } from "@/types";

import { presetSearchText } from "../preset-editor-model";
import { matchesSearch, type ResourceViewMode } from "../resource-model";
import { PresetEditorDialog } from "./PresetEditorDialog";
import { ResourceList, ResourceListButton } from "./ResourceEditorPrimitives";

export function PresetWorkspace({
  kind,
  presets,
  pending,
  error,
  search,
  newRequest,
  viewMode,
  categorySuggestions,
  defaultModel,
}: {
  kind: PromptPresetKindDto;
  presets: ReadonlyArray<PromptPresetDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
  categorySuggestions: ReadonlyArray<string>;
  defaultModel: ImageModelDto;
}) {
  const { t } = useTranslation("resources");
  const [editorPreset, setEditorPreset] = useState<PromptPresetDto | null | undefined>(undefined);
  const previousNewRequest = useRef(newRequest);
  const filtered = useMemo(
    () =>
      presets.filter((preset) =>
        matchesSearch(
          search,
          preset.name,
          preset.category,
          preset.description,
          presetSearchText(preset),
        ),
      ),
    [presets, search],
  );

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setEditorPreset(null);
  }, [newRequest]);

  return (
    <>
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptPresets")}
        viewMode={viewMode}
      >
        {filtered.map((preset) => (
          <ResourceListButton
            key={preset.preset_id}
            selected={editorPreset?.preset_id === preset.preset_id}
            title={preset.name}
            detail={preset.category ?? t("preset")}
            description={preset.description ?? presetSearchText(preset)}
            preview={preset.preview}
            viewMode={viewMode}
            onClick={() => setEditorPreset(preset)}
          />
        ))}
      </ResourceList>
      {editorPreset !== undefined ? (
        <PresetEditorDialog
          key={editorPreset?.preset_id ?? `new-${newRequest}`}
          kind={kind}
          preset={editorPreset}
          categorySuggestions={categorySuggestions}
          defaultModel={defaultModel}
          onClose={() => setEditorPreset(undefined)}
        />
      ) : null}
    </>
  );
}
