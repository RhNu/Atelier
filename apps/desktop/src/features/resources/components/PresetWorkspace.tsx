/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useEffect, useMemo, useRef, useState, type DragEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type { ImageModelDto, PromptPresetDto, PromptPresetKindDto } from "@/types";

import { presetSearchText } from "../preset-editor-model";
import { matchesSearch, type ResourceViewMode } from "../resource-model";
import { useSelectedPromptResource } from "../selected-prompt-resources";
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
  defaultModel,
  onResourceDragStart,
  folderItems,
  currentFolderId = null,
  currentFolderPath = "",
}: {
  kind: PromptPresetKindDto;
  presets: ReadonlyArray<PromptPresetDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
  defaultModel: ImageModelDto;
  onResourceDragStart?: (event: DragEvent, resourceId: string) => void;
  folderItems?: ReactNode;
  currentFolderId?: string | null;
  currentFolderPath?: string;
}) {
  const { t } = useTranslation("resources");
  const [editorPreset, setEditorPreset] = useState<PromptPresetDto | null | undefined>(undefined);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  useSelectedPromptResource(
    presets.some((item) => item.preset_id === selectedId) ? selectedId : null,
  );
  const previousNewRequest = useRef(newRequest);
  const filtered = useMemo(
    () =>
      presets.filter((preset) =>
        matchesSearch(
          search,
          preset.display_name,
          preset.path,
          ...preset.aliases,
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
    setSelectedId(null);
  }, [newRequest]);

  return (
    <>
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptPresets")}
        folderItems={folderItems}
        viewMode={viewMode}
      >
        {filtered.map((preset) => (
          <ResourceListButton
            key={preset.preset_id}
            selected={selectedId === preset.preset_id}
            title={preset.display_name}
            detail={preset.path || t("preset")}
            description={preset.description ?? presetSearchText(preset)}
            preview={preset.preview}
            viewMode={viewMode}
            onDragStart={
              onResourceDragStart
                ? (event) => onResourceDragStart(event, preset.preset_id)
                : undefined
            }
            onClick={() => {
              setSelectedId(preset.preset_id);
              setEditorPreset(preset);
            }}
          />
        ))}
      </ResourceList>
      {editorPreset !== undefined ? (
        <PresetEditorDialog
          key={editorPreset?.preset_id ?? `new-${newRequest}`}
          kind={kind}
          preset={editorPreset}
          defaultModel={defaultModel}
          initialFolderId={currentFolderId}
          initialFolderPath={currentFolderPath}
          onClose={() => setEditorPreset(undefined)}
        />
      ) : null}
    </>
  );
}
