/* eslint-disable max-lines, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import {
  Archive,
  Download,
  Eye,
  FilePlus2,
  Import,
  Plus,
  Save,
  Search,
  Sparkles,
  Trash2,
} from "lucide-react";
import { Children, useEffect, useMemo, useState, type ReactNode } from "react";

import {
  AppButton,
  AppPanel,
  AppTabs,
  AppToolbar,
  EmptyState,
  ResourceImage,
} from "../../components/ui";
import { resourceImageToDataUrl } from "../../platform/atelier";
import type {
  CompiledPromptDto,
  PromptChunkDto,
  PromptPresetDto,
  PromptPresetKindDto,
  ResourceRefDto,
  UpsertPromptChunkRequestDto,
  UpsertPromptPresetRequestDto,
  VibeDocumentEntryDto,
} from "../../types";
import {
  useCompilePromptPreviewMutation,
  useDeletePromptChunkMutation,
  useDeletePromptPresetMutation,
  useEnsureVibeEncodingFromSourceMutation,
  useExportVibeDocumentMutation,
  useImportEmbeddedPngVibeDocumentsMutation,
  useImportVibeDocumentsMutation,
  usePromptChunksQuery,
  usePromptPresetsQuery,
  useRenameVibeDocumentMutation,
  useResourceImageQuery,
  useSetVibeDocumentHiddenMutation,
  useUpsertPromptChunkMutation,
  useUpsertPromptPresetMutation,
  useVibeDocumentsQuery,
} from "./data/useResourcesData";

type ResourceTab = "chunks" | "main-presets" | "character-presets" | "vibe";
type ChunkDraft = UpsertPromptChunkRequestDto;
type PresetDraft = UpsertPromptPresetRequestDto;

const RESOURCE_TABS = [
  { value: "chunks", label: "Prompt Chunks" },
  { value: "main-presets", label: "Main Presets" },
  { value: "character-presets", label: "Character Presets" },
  { value: "vibe", label: "Vibe" },
] as const;
const EMPTY_CHUNKS: ReadonlyArray<PromptChunkDto> = [];
const EMPTY_PRESETS: ReadonlyArray<PromptPresetDto> = [];
const EMPTY_VIBES: ReadonlyArray<VibeDocumentEntryDto> = [];

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function ResourcesPage() {
  const [tab, setTab] = useState<ResourceTab>("chunks");
  const [search, setSearch] = useState("");
  const [includeHiddenVibes, setIncludeHiddenVibes] = useState(false);
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

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Resources</p>
          <h1 className="text-lg font-semibold text-white">Prompt and Vibe Workspace</h1>
        </div>
        <SearchField value={search} onChange={setSearch} />
      </AppToolbar>

      <div className="flex min-h-0 flex-1 flex-col gap-3 p-3">
        <div className="flex items-center justify-between gap-3">
          <AppTabs value={tab} tabs={RESOURCE_TABS} onChange={(value) => setTab(parseTab(value))} />
          <span className="text-xs text-app-muted">{tabSummary(tab)}</span>
        </div>
        {tab === "chunks" ? (
          <ChunkWorkspace
            chunks={chunksQuery.data?.items ?? EMPTY_CHUNKS}
            pending={chunksQuery.isPending}
            error={chunksQuery.isError ? formatError(chunksQuery.error) : null}
            search={search}
          />
        ) : null}
        {tab === "main-presets" ? (
          <PresetWorkspace
            kind="main"
            presets={mainPresetsQuery.data?.items ?? EMPTY_PRESETS}
            pending={mainPresetsQuery.isPending}
            error={mainPresetsQuery.isError ? formatError(mainPresetsQuery.error) : null}
            search={search}
          />
        ) : null}
        {tab === "character-presets" ? (
          <PresetWorkspace
            kind="character"
            presets={characterPresetsQuery.data?.items ?? EMPTY_PRESETS}
            pending={characterPresetsQuery.isPending}
            error={characterPresetsQuery.isError ? formatError(characterPresetsQuery.error) : null}
            search={search}
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
          />
        ) : null}
      </div>
    </div>
  );
}

function SearchField({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <label className="flex items-center gap-2 border border-app-border bg-app-panel px-3 py-1 text-sm text-app-muted">
      <Search aria-hidden="true" className="size-4" />
      <input
        aria-label="Search resources"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="Search resources"
        className="h-7 w-64 bg-transparent text-app-text outline-none placeholder:text-app-muted"
      />
    </label>
  );
}

function ChunkWorkspace({
  chunks,
  pending,
  error,
  search,
}: {
  chunks: ReadonlyArray<PromptChunkDto>;
  pending: boolean;
  error: string | null;
  search: string;
}) {
  const filtered = useMemo(
    () => chunks.filter((chunk) => matchesSearch(search, chunk.key, chunk.content, chunk.category)),
    [chunks, search],
  );
  const [draft, setDraft] = useState<ChunkDraft>(blankChunkDraft());
  const upsertMutation = useUpsertPromptChunkMutation();
  const deleteMutation = useDeletePromptChunkMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(normalizeChunkDraft(draft))
      .then((saved) => setDraft(chunkToDraft(saved)))
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function remove() {
    if (!draft.chunk_id) {
      return;
    }
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ chunk_id: draft.chunk_id })
      .then(() => setDraft(blankChunkDraft()))
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({ prompt: draft.content, max_depth: 8 })
      .then(setPreview)
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }

  return (
    <ResourceEditorLayout
      list={
        <ResourceList pending={pending} error={error} emptyTitle="No prompt chunks">
          {filtered.map((chunk) => (
            <ResourceListButton
              key={chunk.chunk_id}
              selected={draft.chunk_id === chunk.chunk_id}
              title={chunk.key}
              detail={chunk.category ?? "Uncategorized"}
              preview={chunk.preview}
              onClick={() => {
                setDraft(chunkToDraft(chunk));
                setPreview(null);
              }}
            />
          ))}
        </ResourceList>
      }
      editor={
        <EditorPanel
          title="Prompt Chunk"
          subtitle="Reusable @chunk(...) source"
          error={errorMessage}
          actions={
            <EditorActions
              canDelete={Boolean(draft.chunk_id)}
              saving={upsertMutation.isPending}
              deleting={deleteMutation.isPending}
              onNew={() => {
                setDraft(blankChunkDraft());
                setPreview(null);
              }}
              onSave={save}
              onDelete={remove}
            />
          }
        >
          <TextInput
            label="Key"
            value={draft.key}
            onChange={(key) => setDraft({ ...draft, key })}
          />
          <TextInput
            label="Category"
            value={draft.category ?? ""}
            onChange={(category) => setDraft({ ...draft, category: nullableText(category) })}
          />
          <TextInput
            label="Description"
            value={draft.description ?? ""}
            onChange={(description) =>
              setDraft({ ...draft, description: nullableText(description) })
            }
          />
          <TextArea
            label="Content"
            value={draft.content}
            minRows="min-h-40"
            onChange={(content) => setDraft({ ...draft, content })}
          />
          <PreviewSlot resource={draft.preview} label="Chunk preview" />
          <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
            <Eye aria-hidden="true" className="size-4" />
            Compile preview
          </AppButton>
          <CompiledPreview preview={preview} />
        </EditorPanel>
      }
    />
  );
}

function PresetWorkspace({
  kind,
  presets,
  pending,
  error,
  search,
}: {
  kind: PromptPresetKindDto;
  presets: ReadonlyArray<PromptPresetDto>;
  pending: boolean;
  error: string | null;
  search: string;
}) {
  const filtered = useMemo(
    () =>
      presets.filter((preset) =>
        matchesSearch(search, preset.name, preset.category, preset.description, preset.before),
      ),
    [presets, search],
  );
  const [draft, setDraft] = useState<PresetDraft>(blankPresetDraft(kind));
  const upsertMutation = useUpsertPromptPresetMutation();
  const deleteMutation = useDeletePromptPresetMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const mainPreset = kind === "main";

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(normalizePresetDraft(draft, kind))
      .then((saved) => setDraft(presetToDraft(saved)))
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function remove() {
    if (!draft.preset_id) {
      return;
    }
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ preset_id: draft.preset_id })
      .then(() => setDraft(blankPresetDraft(kind)))
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({ prompt: presetPreviewSource(draft), max_depth: 8 })
      .then(setPreview)
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }

  return (
    <ResourceEditorLayout
      list={
        <ResourceList pending={pending} error={error} emptyTitle="No prompt presets">
          {filtered.map((preset) => (
            <ResourceListButton
              key={preset.preset_id}
              selected={draft.preset_id === preset.preset_id}
              title={preset.name}
              detail={`${preset.enabled ? "Enabled" : "Disabled"} · ${preset.category ?? "Preset"}`}
              preview={preset.preview}
              onClick={() => {
                setDraft(presetToDraft(preset));
                setPreview(null);
              }}
            />
          ))}
        </ResourceList>
      }
      editor={
        <EditorPanel
          title={mainPreset ? "Main Preset" : "Character Preset"}
          subtitle={
            mainPreset ? "Applies global prompt and UC overrides" : "Applies one character prompt"
          }
          error={errorMessage}
          actions={
            <EditorActions
              canDelete={Boolean(draft.preset_id)}
              saving={upsertMutation.isPending}
              deleting={deleteMutation.isPending}
              onNew={() => {
                setDraft(blankPresetDraft(kind));
                setPreview(null);
              }}
              onSave={save}
              onDelete={remove}
            />
          }
        >
          <div className="grid grid-cols-2 gap-3">
            <TextInput
              label="Name"
              value={draft.name}
              onChange={(name) => setDraft({ ...draft, name })}
            />
            <NumberInput
              label="Order"
              value={draft.order}
              onChange={(order) => setDraft({ ...draft, order })}
            />
          </div>
          <CheckboxField
            label="Enabled"
            checked={draft.enabled}
            onChange={(enabled) => setDraft({ ...draft, enabled })}
          />
          <TextInput
            label="Category"
            value={draft.category ?? ""}
            onChange={(category) => setDraft({ ...draft, category: nullableText(category) })}
          />
          <TextInput
            label="Description"
            value={draft.description ?? ""}
            onChange={(description) =>
              setDraft({ ...draft, description: nullableText(description) })
            }
          />
          <PresetFields draft={draft} setDraft={setDraft} mainPreset={mainPreset} />
          <PreviewSlot resource={draft.preview} label="Preset preview" />
          <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
            <Eye aria-hidden="true" className="size-4" />
            Compile preset fields
          </AppButton>
          <CompiledPreview preview={preview} />
        </EditorPanel>
      }
    />
  );
}

function PresetFields({
  draft,
  setDraft,
  mainPreset,
}: {
  draft: PresetDraft;
  setDraft: (draft: PresetDraft) => void;
  mainPreset: boolean;
}) {
  return (
    <>
      <TextArea
        label="Before"
        value={draft.before}
        minRows="min-h-24"
        onChange={(before) => setDraft({ ...draft, before })}
      />
      <TextArea
        label="After"
        value={draft.after}
        minRows="min-h-20"
        onChange={(after) => setDraft({ ...draft, after })}
      />
      <TextArea
        label="Replace"
        value={draft.replace}
        minRows="min-h-20"
        onChange={(replace) => setDraft({ ...draft, replace })}
      />
      <div className="grid grid-cols-3 gap-3">
        <TextArea
          label="UC before"
          value={draft.uc_before}
          minRows="min-h-20"
          onChange={(uc_before) => setDraft({ ...draft, uc_before })}
        />
        <TextArea
          label="UC after"
          value={draft.uc_after}
          minRows="min-h-20"
          onChange={(uc_after) => setDraft({ ...draft, uc_after })}
        />
        <TextArea
          label="UC replace"
          value={draft.uc_replace}
          minRows="min-h-20"
          onChange={(uc_replace) => setDraft({ ...draft, uc_replace })}
        />
      </div>
      {mainPreset ? (
        <div className="grid grid-cols-2 gap-3">
          <TextInput
            label="Quality override"
            value={draft.quality_override ?? ""}
            onChange={(quality_override) =>
              setDraft({ ...draft, quality_override: nullableText(quality_override) })
            }
          />
          <TextInput
            label="UC preset override"
            value={draft.uc_preset_override ?? ""}
            onChange={(uc_preset_override) =>
              setDraft({ ...draft, uc_preset_override: nullableText(uc_preset_override) })
            }
          />
        </div>
      ) : null}
    </>
  );
}

function VibeWorkspace({
  vibes,
  pending,
  error,
  search,
  includeHidden,
  onIncludeHiddenChange,
}: {
  vibes: ReadonlyArray<VibeDocumentEntryDto>;
  pending: boolean;
  error: string | null;
  search: string;
  includeHidden: boolean;
  onIncludeHiddenChange: (value: boolean) => void;
}) {
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const filtered = useMemo(
    () => vibes.filter((vibe) => matchesSearch(search, vibe.display_name, vibe.vibe_id)),
    [search, vibes],
  );
  const importVibeMutation = useImportVibeDocumentsMutation();
  const importPngMutation = useImportEmbeddedPngVibeDocumentsMutation();
  const ensureEncodingMutation = useEnsureVibeEncodingFromSourceMutation();
  const exportMutation = useExportVibeDocumentMutation();
  const renameMutation = useRenameVibeDocumentMutation();
  const hideMutation = useSetVibeDocumentHiddenMutation();
  const commandError =
    importVibeMutation.error ??
    importPngMutation.error ??
    ensureEncodingMutation.error ??
    exportMutation.error ??
    renameMutation.error ??
    hideMutation.error;

  function exportSelected() {
    if (selectedIds.length === 0) {
      return;
    }
    exportMutation.mutate({
      vibe_ids: selectedIds,
      format: selectedIds.length === 1 ? "naiv4vibe" : "naiv4vibebundle",
    });
  }
  function ensureEncodingHandler(vibe: VibeDocumentEntryDto) {
    const sourceImage = vibe.source_image;
    return sourceImage
      ? () =>
          ensureEncodingMutation.mutate({
            vibeId: vibe.vibe_id,
            sourceImage,
          })
      : null;
  }

  return (
    <AppPanel className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Vibe Documents</h2>
          <p className="text-xs text-app-muted">Managed documents and cached encodings</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <CheckboxField
            label="Show hidden"
            checked={includeHidden}
            onChange={onIncludeHiddenChange}
          />
          <AppButton
            variant="secondary"
            onClick={() => importVibeMutation.mutate()}
            disabled={importVibeMutation.isPending}
          >
            <Import aria-hidden="true" className="size-4" />
            Import .naiv4vibe
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={() => importPngMutation.mutate()}
            disabled={importPngMutation.isPending}
          >
            <FilePlus2 aria-hidden="true" className="size-4" />
            Import PNG Vibe
          </AppButton>
          <AppButton
            variant="secondary"
            onClick={exportSelected}
            disabled={exportMutation.isPending || selectedIds.length === 0}
          >
            <Download aria-hidden="true" className="size-4" />
            Export selected
          </AppButton>
        </div>
      </header>
      {commandError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {formatError(commandError)}
        </p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {pending ? (
          <EmptyState title="Loading Vibe documents" />
        ) : error ? (
          <EmptyState title="Vibe unavailable" description={error} />
        ) : filtered.length === 0 ? (
          <EmptyState title="No Vibe documents" />
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-3">
            {filtered.map((vibe) => (
              <VibeCard
                key={vibe.vibe_id}
                vibe={vibe}
                selected={selectedIds.includes(vibe.vibe_id)}
                exportPending={exportMutation.isPending}
                encodePending={ensureEncodingMutation.isPending}
                onToggleSelected={(selected) =>
                  setSelectedIds((current) =>
                    selected
                      ? [...current, vibe.vibe_id]
                      : current.filter((id) => id !== vibe.vibe_id),
                  )
                }
                onExport={() =>
                  exportMutation.mutate({
                    vibe_ids: [vibe.vibe_id],
                    format: "naiv4vibe",
                  })
                }
                onRename={(display_name) =>
                  renameMutation.mutate({ vibe_id: vibe.vibe_id, display_name })
                }
                onHide={() => hideMutation.mutate({ vibe_id: vibe.vibe_id, hidden: !vibe.hidden })}
                onEnsureEncoding={ensureEncodingHandler(vibe)}
              />
            ))}
          </div>
        )}
      </div>
    </AppPanel>
  );
}

function VibeCard({
  vibe,
  selected,
  exportPending,
  encodePending,
  onToggleSelected,
  onExport,
  onRename,
  onHide,
  onEnsureEncoding,
}: {
  vibe: VibeDocumentEntryDto;
  selected: boolean;
  exportPending: boolean;
  encodePending: boolean;
  onToggleSelected: (selected: boolean) => void;
  onExport: () => void;
  onRename: (displayName: string) => void;
  onHide: () => void;
  onEnsureEncoding: (() => void) | null;
}) {
  const [name, setName] = useState(vibe.display_name);
  useEffect(() => {
    setName(vibe.display_name);
  }, [vibe.display_name]);
  return (
    <article className="grid gap-3 border border-app-border bg-app-surface p-3">
      <PreviewSlot resource={vibe.preview ?? vibe.source_image} label={vibe.display_name} />
      <label className="flex items-center gap-2 text-xs text-app-muted">
        <input
          aria-label={`Select ${vibe.display_name}`}
          type="checkbox"
          checked={selected}
          onChange={(event) => onToggleSelected(event.target.checked)}
        />
        Select
      </label>
      <TextInput label="Local display name" value={name} onChange={setName} />
      <div className="grid grid-cols-3 gap-2">
        <AppButton variant="secondary" onClick={() => onRename(name)}>
          <Save aria-hidden="true" className="size-4" />
          Rename
        </AppButton>
        <AppButton variant="secondary" onClick={onExport} disabled={exportPending}>
          <Download aria-hidden="true" className="size-4" />
          Export
        </AppButton>
        <AppButton variant="ghost" onClick={onHide}>
          <Archive aria-hidden="true" className="size-4" />
          {vibe.hidden ? "Unhide" : "Hide"}
        </AppButton>
      </div>
      {onEnsureEncoding ? (
        <AppButton variant="secondary" onClick={onEnsureEncoding} disabled={encodePending}>
          <Sparkles aria-hidden="true" className="size-4" />
          Encode source
        </AppButton>
      ) : null}
      <div className="grid gap-1 text-xs text-app-muted">
        <span>{vibe.available_model_keys.length} models</span>
        <span>{vibe.available_encoding_configs.length} encoding configs</span>
        <span>{vibe.encodings.length} cached encodings</span>
        {vibe.hidden ? <span className="text-amber-200">Hidden</span> : null}
      </div>
    </article>
  );
}

function ResourceEditorLayout({ list, editor }: { list: ReactNode; editor: ReactNode }) {
  return (
    <div className="grid min-h-0 flex-1 grid-cols-[360px_minmax(0,1fr)] gap-3">
      {list}
      {editor}
    </div>
  );
}

function ResourceList({
  pending,
  error,
  emptyTitle,
  children,
}: {
  pending: boolean;
  error: string | null;
  emptyTitle: string;
  children: ReactNode;
}) {
  return (
    <AppPanel className="min-h-0 overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Library</h2>
      </header>
      <div className="min-h-0 overflow-auto p-3">
        {pending ? (
          <EmptyState title="Loading resources" />
        ) : error ? (
          <EmptyState title="Resources unavailable" description={error} />
        ) : Children.count(children) === 0 ? (
          <EmptyState title={emptyTitle} />
        ) : (
          <div className="grid gap-2">{children}</div>
        )}
      </div>
    </AppPanel>
  );
}

function ResourceListButton({
  selected,
  title,
  detail,
  preview,
  onClick,
}: {
  selected: boolean;
  title: string;
  detail: string;
  preview: ResourceRefDto | null;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "grid grid-cols-[56px_minmax(0,1fr)] gap-3 border p-2 text-left",
        selected
          ? "border-brand-400/70 bg-brand-500/10"
          : "border-app-border bg-app-surface hover:border-brand-400/60",
      ].join(" ")}
    >
      <PreviewSlot resource={preview} label={title} compact />
      <span className="min-w-0">
        <span className="block truncate text-sm font-semibold text-app-text">{title}</span>
        <span className="mt-1 block truncate text-xs text-app-muted">{detail}</span>
      </span>
    </button>
  );
}

function EditorPanel({
  title,
  subtitle,
  error,
  actions,
  children,
}: {
  title: string;
  subtitle: string;
  error: string | null;
  actions: ReactNode;
  children: ReactNode;
}) {
  return (
    <AppPanel className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <div>
          <h2 className="text-sm font-semibold text-white">{title}</h2>
          <p className="text-xs text-app-muted">{subtitle}</p>
        </div>
        {actions}
      </header>
      {error ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {error}
        </p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="grid gap-3">{children}</div>
      </div>
    </AppPanel>
  );
}

function EditorActions({
  canDelete,
  saving,
  deleting,
  onNew,
  onSave,
  onDelete,
}: {
  canDelete: boolean;
  saving: boolean;
  deleting: boolean;
  onNew: () => void;
  onSave: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="flex gap-2">
      <AppButton variant="ghost" onClick={onNew}>
        <Plus aria-hidden="true" className="size-4" />
        New
      </AppButton>
      <AppButton variant="secondary" onClick={onSave} disabled={saving}>
        <Save aria-hidden="true" className="size-4" />
        Save
      </AppButton>
      <AppButton variant="danger" onClick={onDelete} disabled={!canDelete || deleting}>
        <Trash2 aria-hidden="true" className="size-4" />
        Delete
      </AppButton>
    </div>
  );
}

function PreviewSlot({
  resource,
  label,
  compact = false,
}: {
  resource: ResourceRefDto | null;
  label: string;
  compact?: boolean;
}) {
  const imageQuery = useResourceImageQuery(resource);
  const src = imageQuery.data ? resourceImageToDataUrl(imageQuery.data) : null;
  return (
    <ResourceImage
      src={src}
      alt={label}
      fallbackLabel={compact ? "" : "No preview"}
      className={
        compact
          ? "size-14 border border-app-border"
          : "aspect-video w-full border border-app-border"
      }
    />
  );
}

function CompiledPreview({ preview }: { preview: CompiledPromptDto | null }) {
  if (!preview) {
    return null;
  }
  return (
    <article className="border border-app-border bg-black/20 p-3">
      <p className="text-xs font-semibold text-app-muted uppercase">Compiled preview</p>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {preview.expanded_prompt || "Empty"}
      </p>
      <p className="mt-2 text-xs text-app-muted">
        {preview.trace.function_calls.length} function calls
      </p>
    </article>
  );
}

function TextInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

function NumberInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        type="number"
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || 0)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

function TextArea({
  label,
  value,
  minRows,
  onChange,
}: {
  label: string;
  value: string;
  minRows: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <textarea
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={[
          minRows,
          "resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400",
        ].join(" ")}
      />
    </label>
  );
}

function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex h-9 items-center gap-2 border border-app-border bg-black/20 px-3 text-sm text-app-text">
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  );
}

function blankChunkDraft(): ChunkDraft {
  return {
    chunk_id: null,
    key: "",
    content: "",
    category: null,
    description: null,
    preview: null,
  };
}

function chunkToDraft(chunk: PromptChunkDto): ChunkDraft {
  return {
    chunk_id: chunk.chunk_id,
    key: chunk.key,
    content: chunk.content,
    category: chunk.category,
    description: chunk.description,
    preview: chunk.preview,
  };
}

function normalizeChunkDraft(draft: ChunkDraft): ChunkDraft {
  return {
    ...draft,
    key: draft.key.trim(),
    category: nullableText(draft.category ?? ""),
    description: nullableText(draft.description ?? ""),
  };
}

function blankPresetDraft(kind: PromptPresetKindDto): PresetDraft {
  return {
    preset_id: null,
    kind,
    name: "",
    category: null,
    description: null,
    order: 0,
    enabled: true,
    before: "",
    after: "",
    replace: "",
    uc_before: "",
    uc_after: "",
    uc_replace: "",
    quality_override: null,
    uc_preset_override: null,
    preview: null,
  };
}

function presetToDraft(preset: PromptPresetDto): PresetDraft {
  return {
    preset_id: preset.preset_id,
    kind: preset.kind,
    name: preset.name,
    category: preset.category,
    description: preset.description,
    order: preset.order,
    enabled: preset.enabled,
    before: preset.before,
    after: preset.after,
    replace: preset.replace,
    uc_before: preset.uc_before,
    uc_after: preset.uc_after,
    uc_replace: preset.uc_replace,
    quality_override: preset.quality_override,
    uc_preset_override: preset.uc_preset_override,
    preview: preset.preview,
  };
}

function normalizePresetDraft(draft: PresetDraft, kind: PromptPresetKindDto): PresetDraft {
  return {
    ...draft,
    kind,
    name: draft.name.trim(),
    category: nullableText(draft.category ?? ""),
    description: nullableText(draft.description ?? ""),
    quality_override: kind === "main" ? nullableText(draft.quality_override ?? "") : null,
    uc_preset_override: kind === "main" ? nullableText(draft.uc_preset_override ?? "") : null,
  };
}

function presetPreviewSource(draft: PresetDraft): string {
  return [
    draft.before,
    draft.replace,
    draft.after,
    draft.uc_before,
    draft.uc_replace,
    draft.uc_after,
  ]
    .filter((part) => part.trim().length > 0)
    .join("\n");
}

function nullableText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function matchesSearch(search: string, ...values: Array<string | null>): boolean {
  const needle = search.trim().toLowerCase();
  if (!needle) {
    return true;
  }
  return values.some((value) => value?.toLowerCase().includes(needle));
}

function parseTab(value: string): ResourceTab {
  switch (value) {
    case "main-presets":
    case "character-presets":
    case "vibe":
      return value;
    default:
      return "chunks";
  }
}

function tabSummary(tab: ResourceTab): string {
  switch (tab) {
    case "chunks":
      return "Reusable @chunk(...) prompt fragments";
    case "main-presets":
      return "Global prompt presets and generation overrides";
    case "character-presets":
      return "Character prompt presets without generation overrides";
    case "vibe":
      return "NovelAI Vibe documents and encodings";
  }
}
