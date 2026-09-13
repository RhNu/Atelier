/* eslint-disable react-perf/jsx-no-new-array-as-prop */
import { LayoutGrid, List, Plus } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppHelpMarker, AppIconButton, AppSelect, AppTabs } from "@/components/ui";
import { useImageModelCatalog } from "@/features/generation/data/useImageModelCatalog";
import {
  generationModelDisplayNames,
  toImageModel,
} from "@/features/generation/model/generation-options";
import type { ImageModelDto, PromptChunkDto, PromptPresetDto, VibeDocumentEntryDto } from "@/types";

import { ChunkWorkspace } from "./components/ChunkWorkspace";
import { LibraryNavigator } from "./components/LibraryNavigator";
import { PresetWorkspace } from "./components/PresetWorkspace";
import { SearchField } from "./components/ResourceEditorPrimitives";
import { VibeWorkspace } from "./components/VibeWorkspace";
import {
  usePromptChunksQuery,
  usePromptPresetsQuery,
  useVibeDocumentsQuery,
} from "./data/useResourcesData";
import { formatError, parseTab, type ResourceTab, type ResourceViewMode } from "./resource-model";

const EMPTY_CHUNKS: ReadonlyArray<PromptChunkDto> = [];
const EMPTY_PRESETS: ReadonlyArray<PromptPresetDto> = [];
const EMPTY_VIBES: ReadonlyArray<VibeDocumentEntryDto> = [];
const TAB_SUMMARY_KEYS = {
  chunks: "chunks",
  "main-presets": "mainPresets",
  "character-presets": "characterPresets",
  vibe: "vibe",
} as const;

export function ResourcesPage() {
  const [tab, setTab] = useState<ResourceTab>("chunks");
  const [viewMode, setViewMode] = useState<ResourceViewMode>("grid");
  const [search, setSearch] = useState("");
  const [includeHiddenVibes, setIncludeHiddenVibes] = useState(false);
  const [newRequest, setNewRequest] = useState(0);
  const [modelFilter, setModelFilter] = useState<ImageModelDto | null>(null);
  const chunksQuery = usePromptChunksQuery({ offset: 0, limit: 200, model: modelFilter });
  const mainPresetsQuery = usePromptPresetsQuery({
    kind: "main",
    offset: 0,
    limit: 200,
    model: modelFilter,
  });
  const characterPresetsQuery = usePromptPresetsQuery({
    kind: "character",
    offset: 0,
    limit: 200,
    model: modelFilter,
  });
  const vibesQuery = useVibeDocumentsQuery({
    offset: 0,
    limit: 200,
    include_hidden: includeHiddenVibes,
    model: modelFilter,
  });
  const handleTabChange = useCallback((value: string) => setTab(parseTab(value)), []);
  const handleNew = useCallback(() => setNewRequest((value) => value + 1), []);
  const handleListView = useCallback(() => setViewMode("list"), []);
  const handleGridView = useCallback(() => setViewMode("grid"), []);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col">
        <ResourcesToolbar
          tab={tab}
          viewMode={viewMode}
          search={search}
          modelFilter={modelFilter}
          onTabChange={handleTabChange}
          onListView={handleListView}
          onGridView={handleGridView}
          onSearchChange={setSearch}
          onModelFilterChange={setModelFilter}
          onNew={handleNew}
        />
        {tab === "chunks" ? (
          <LibraryNavigator namespace="prompt_chunk" search={search} viewMode={viewMode}>
            {(visibleIds, startDrag, currentFolder, _resources, folderItems) => (
              <ChunkWorkspace
                chunks={(chunksQuery.data?.items ?? EMPTY_CHUNKS).filter((chunk) =>
                  visibleIds.has(chunk.chunk_id),
                )}
                pending={chunksQuery.isPending}
                error={chunksQuery.isError ? formatError(chunksQuery.error) : null}
                search=""
                newRequest={newRequest}
                viewMode={viewMode}
                defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
                onResourceDragStart={startDrag}
                folderItems={folderItems}
                currentFolderId={currentFolder.id}
                currentFolderPath={currentFolder.path}
              />
            )}
          </LibraryNavigator>
        ) : null}
        {tab === "main-presets" ? (
          <LibraryNavigator namespace="main_preset" search={search} viewMode={viewMode}>
            {(visibleIds, startDrag, currentFolder, _resources, folderItems) => (
              <PresetWorkspace
                kind="main"
                presets={(mainPresetsQuery.data?.items ?? EMPTY_PRESETS).filter((preset) =>
                  visibleIds.has(preset.preset_id),
                )}
                pending={mainPresetsQuery.isPending}
                error={mainPresetsQuery.isError ? formatError(mainPresetsQuery.error) : null}
                search=""
                newRequest={newRequest}
                viewMode={viewMode}
                defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
                onResourceDragStart={startDrag}
                folderItems={folderItems}
                currentFolderId={currentFolder.id}
                currentFolderPath={currentFolder.path}
              />
            )}
          </LibraryNavigator>
        ) : null}
        {tab === "character-presets" ? (
          <LibraryNavigator namespace="character_preset" search={search} viewMode={viewMode}>
            {(visibleIds, startDrag, currentFolder, _resources, folderItems) => (
              <PresetWorkspace
                kind="character"
                presets={(characterPresetsQuery.data?.items ?? EMPTY_PRESETS).filter((preset) =>
                  visibleIds.has(preset.preset_id),
                )}
                pending={characterPresetsQuery.isPending}
                error={
                  characterPresetsQuery.isError ? formatError(characterPresetsQuery.error) : null
                }
                search=""
                newRequest={newRequest}
                viewMode={viewMode}
                defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
                onResourceDragStart={startDrag}
                folderItems={folderItems}
                currentFolderId={currentFolder.id}
                currentFolderPath={currentFolder.path}
              />
            )}
          </LibraryNavigator>
        ) : null}
        {tab === "vibe" ? (
          <LibraryNavigator namespace="vibe" search={search} viewMode={viewMode}>
            {(visibleIds, startDrag, _currentFolder, libraryResources, folderItems) => (
              <VibeWorkspace
                vibes={(vibesQuery.data?.items ?? EMPTY_VIBES).filter((vibe) =>
                  visibleIds.has(vibe.vibe_id),
                )}
                pending={vibesQuery.isPending}
                error={vibesQuery.isError ? formatError(vibesQuery.error) : null}
                search=""
                includeHidden={includeHiddenVibes}
                onIncludeHiddenChange={setIncludeHiddenVibes}
                viewMode={viewMode}
                onResourceDragStart={startDrag}
                libraryResources={libraryResources}
                folderItems={folderItems}
              />
            )}
          </LibraryNavigator>
        ) : null}
      </div>
    </div>
  );
}

type ResourcesToolbarProps = {
  tab: ResourceTab;
  viewMode: ResourceViewMode;
  search: string;
  modelFilter: ImageModelDto | null;
  onTabChange: (value: string) => void;
  onListView: () => void;
  onGridView: () => void;
  onSearchChange: (value: string) => void;
  onModelFilterChange: (model: ImageModelDto | null) => void;
  onNew: () => void;
};

function ResourcesToolbar({
  tab,
  viewMode,
  search,
  modelFilter,
  onTabChange,
  onListView,
  onGridView,
  onSearchChange,
  onModelFilterChange,
  onNew,
}: ResourcesToolbarProps) {
  const { t } = useTranslation("resources");
  const modelCatalog = useImageModelCatalog();
  const resourceTabs = useMemo(
    () => [
      { value: "chunks", label: t("promptChunks") },
      { value: "main-presets", label: t("mainPresets") },
      { value: "character-presets", label: t("characterPresets") },
      { value: "vibe", label: "Vibe" },
    ],
    [t],
  );
  const modelOptions = useMemo(
    () => [
      { value: "all", label: t("allModels") },
      ...(modelCatalog.data ?? []).map(({ model }) => ({
        value: model,
        label: generationModelDisplayNames[model],
      })),
    ],
    [modelCatalog.data, t],
  );
  const handleModelFilterChange = useCallback(
    (value: string) => onModelFilterChange(value === "all" ? null : toImageModel(value)),
    [onModelFilterChange],
  );
  return (
    <div className="flex min-h-12 items-center justify-between gap-3 border-b border-app-border bg-app-panel px-3 py-2">
      <div className="flex min-w-0 items-center gap-3">
        <AppTabs value={tab} tabs={resourceTabs} onChange={onTabChange} />
        <AppHelpMarker
          label={t("tabHelp")}
          content={t(`tabSummary.${TAB_SUMMARY_KEYS[tab]}`)}
          hoverOnly
        />
      </div>
      <div className="flex items-center gap-2">
        <fieldset className="flex border border-app-border bg-app-surface">
          <legend className="sr-only">{t("viewMode")}</legend>
          <AppIconButton
            icon={List}
            label={t("listView")}
            size="sm"
            selected={viewMode === "list"}
            aria-pressed={viewMode === "list"}
            onClick={onListView}
          />
          <AppIconButton
            icon={LayoutGrid}
            label={t("gridView")}
            size="sm"
            selected={viewMode === "grid"}
            aria-pressed={viewMode === "grid"}
            onClick={onGridView}
          />
        </fieldset>
        <SearchField value={search} onChange={onSearchChange} />
        <AppSelect
          aria-label={t("modelFilter")}
          value={modelFilter ?? "all"}
          options={modelOptions}
          onValueChange={handleModelFilterChange}
        />
        {tab === "vibe" ? null : (
          <AppButton variant="secondary" onClick={onNew}>
            <Plus aria-hidden="true" className="size-4" />
            {t("new")}
          </AppButton>
        )}
      </div>
    </div>
  );
}
