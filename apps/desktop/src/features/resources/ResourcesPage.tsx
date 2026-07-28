import { LayoutGrid, List, Plus } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppHelpMarker, AppIconButton, AppTabs } from "@/components/ui";
import type { PromptChunkDto, PromptPresetDto, VibeDocumentEntryDto } from "@/types";

import { ChunkWorkspace } from "./components/ChunkWorkspace";
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
  const { t } = useTranslation("resources");
  const resourceTabs = [
    { value: "chunks", label: t("promptChunks") },
    { value: "main-presets", label: t("mainPresets") },
    { value: "character-presets", label: t("characterPresets") },
    { value: "vibe", label: "Vibe" },
  ] as const;
  const [tab, setTab] = useState<ResourceTab>("chunks");
  const [viewMode, setViewMode] = useState<ResourceViewMode>("list");
  const [search, setSearch] = useState("");
  const [includeHiddenVibes, setIncludeHiddenVibes] = useState(false);
  const [newRequest, setNewRequest] = useState(0);
  const chunksQuery = usePromptChunksQuery({ offset: 0, limit: 200 });
  const mainPresetsQuery = usePromptPresetsQuery({
    kind: "main",
    include_disabled: true,
    offset: 0,
    limit: 200,
  });
  const characterPresetsQuery = usePromptPresetsQuery({
    kind: "character",
    include_disabled: true,
    offset: 0,
    limit: 200,
  });
  const vibesQuery = useVibeDocumentsQuery({
    offset: 0,
    limit: 200,
    include_hidden: includeHiddenVibes,
  });
  const handleTabChange = useCallback((value: string) => setTab(parseTab(value)), []);
  const handleNew = useCallback(() => setNewRequest((value) => value + 1), []);
  const handleListView = useCallback(() => setViewMode("list"), []);
  const handleGridView = useCallback(() => setViewMode("grid"), []);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="flex min-h-12 items-center justify-between gap-3 border-b border-app-border bg-app-panel px-3 py-2">
          <div className="flex min-w-0 items-center gap-3">
            <AppTabs value={tab} tabs={resourceTabs} onChange={handleTabChange} />
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
                onClick={handleListView}
              />
              <AppIconButton
                icon={LayoutGrid}
                label={t("gridView")}
                size="sm"
                selected={viewMode === "grid"}
                aria-pressed={viewMode === "grid"}
                onClick={handleGridView}
              />
            </fieldset>
            <SearchField value={search} onChange={setSearch} />
            {tab === "vibe" ? null : (
              <AppButton variant="secondary" onClick={handleNew}>
                <Plus aria-hidden="true" className="size-4" />
                {t("new")}
              </AppButton>
            )}
          </div>
        </div>
        {tab === "chunks" ? (
          <ChunkWorkspace
            chunks={chunksQuery.data?.items ?? EMPTY_CHUNKS}
            pending={chunksQuery.isPending}
            error={chunksQuery.isError ? formatError(chunksQuery.error) : null}
            search={search}
            newRequest={newRequest}
            viewMode={viewMode}
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
