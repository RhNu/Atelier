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
import { PresetWorkspace } from "./components/PresetWorkspace";
import { SearchField } from "./components/ResourceEditorPrimitives";
import { VibeWorkspace } from "./components/VibeWorkspace";
import {
  usePromptChunksQuery,
  usePromptPresetsQuery,
  useVibeDocumentsQuery,
} from "./data/useResourcesData";
import {
  categorySuggestions,
  formatError,
  parseTab,
  type ResourceTab,
  type ResourceViewMode,
} from "./resource-model";

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
  const chunkCategories = categorySuggestions(
    (chunksQuery.data?.items ?? EMPTY_CHUNKS).map((chunk) => chunk.category),
  );
  const mainPresetCategories = categorySuggestions(
    (mainPresetsQuery.data?.items ?? EMPTY_PRESETS).map((preset) => preset.category),
  );
  const characterPresetCategories = categorySuggestions(
    (characterPresetsQuery.data?.items ?? EMPTY_PRESETS).map((preset) => preset.category),
  );

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
          <ChunkWorkspace
            chunks={chunksQuery.data?.items ?? EMPTY_CHUNKS}
            pending={chunksQuery.isPending}
            error={chunksQuery.isError ? formatError(chunksQuery.error) : null}
            search={search}
            newRequest={newRequest}
            viewMode={viewMode}
            categorySuggestions={chunkCategories}
            defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
          />
        ) : null}
        {tab === "main-presets" ? (
          <PresetWorkspace
            kind="main"
            presets={mainPresetsQuery.data?.items ?? EMPTY_PRESETS}
            pending={mainPresetsQuery.isPending}
            error={mainPresetsQuery.isError ? formatError(mainPresetsQuery.error) : null}
            search={search}
            newRequest={newRequest}
            viewMode={viewMode}
            categorySuggestions={mainPresetCategories}
            defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
          />
        ) : null}
        {tab === "character-presets" ? (
          <PresetWorkspace
            kind="character"
            presets={characterPresetsQuery.data?.items ?? EMPTY_PRESETS}
            pending={characterPresetsQuery.isPending}
            error={characterPresetsQuery.isError ? formatError(characterPresetsQuery.error) : null}
            search={search}
            newRequest={newRequest}
            viewMode={viewMode}
            categorySuggestions={characterPresetCategories}
            defaultModel={modelFilter ?? "nai-diffusion-4-5-full"}
          />
        ) : null}
        {tab === "vibe" ? (
          <VibeWorkspace
            vibes={vibesQuery.data?.items ?? EMPTY_VIBES}
            pending={vibesQuery.isPending}
            error={vibesQuery.isError ? formatError(vibesQuery.error) : null}
            search={search}
            includeHidden={includeHiddenVibes}
            onIncludeHiddenChange={setIncludeHiddenVibes}
            viewMode={viewMode}
          />
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
