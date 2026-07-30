/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Check, LibraryBig, Search, Trash2, WandSparkles } from "lucide-react";
import { useDeferredValue, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton, AppModal, AppSelect } from "@/components/ui";
import type { PromptPresetDto } from "@/types";

import { GenerationResourceThumbnail } from "./GenerationResourceThumbnail";

const ALL_CATEGORIES = "__all__";
const UNCATEGORIZED = "__uncategorized__";

type GenerationPresetControlProps = {
  label: string;
  noPresetLabel: string;
  libraryTitle: string;
  presets: ReadonlyArray<PromptPresetDto>;
  selectedPresetId: string | null;
  pending?: boolean;
  compact?: boolean;
  onSelect: (presetId: string) => void;
  onClear: () => void;
  onApply: (preset: PromptPresetDto) => void;
};

export function GenerationPresetControl({
  label,
  noPresetLabel,
  libraryTitle,
  presets,
  selectedPresetId,
  pending = false,
  compact = false,
  onSelect,
  onClear,
  onApply,
}: GenerationPresetControlProps) {
  const { t } = useTranslation("generation");
  const [dialogOpen, setDialogOpen] = useState(false);
  const selectedPreset = presets.find((preset) => preset.preset_id === selectedPresetId) ?? null;
  const displayName = pending ? t("loadingPresets") : (selectedPreset?.name ?? noPresetLabel);

  return (
    <div className="grid gap-1.5">
      {compact ? null : (
        <span className="text-xs font-semibold text-app-muted uppercase">{label}</span>
      )}
      <div className="flex min-w-0 items-center gap-1">
        <input
          aria-label={label}
          value={displayName}
          readOnly
          tabIndex={-1}
          className="h-9 min-w-0 flex-1 border border-app-border bg-black/20 px-3 text-sm text-app-text outline-none"
        />
        <AppIconButton
          icon={LibraryBig}
          label={t("choosePreset", { preset: label })}
          size="sm"
          disabled={pending}
          onClick={() => setDialogOpen(true)}
        />
        <AppIconButton
          icon={Trash2}
          label={t("clearPreset", { preset: label })}
          size="sm"
          variant="danger"
          disabled={!selectedPreset}
          onClick={onClear}
        />
        <AppIconButton
          icon={WandSparkles}
          label={t("applyPreset", { preset: label })}
          size="sm"
          disabled={!selectedPreset}
          onClick={() => selectedPreset && onApply(selectedPreset)}
        />
      </div>

      <PresetLibraryDialog
        open={dialogOpen}
        title={libraryTitle}
        presets={presets}
        selectedPresetId={selectedPresetId}
        onClose={() => setDialogOpen(false)}
        onSelect={onSelect}
      />
    </div>
  );
}

function PresetLibraryDialog({
  open,
  title,
  presets,
  selectedPresetId,
  onClose,
  onSelect,
}: {
  open: boolean;
  title: string;
  presets: ReadonlyArray<PromptPresetDto>;
  selectedPresetId: string | null;
  onClose: () => void;
  onSelect: (presetId: string) => void;
}) {
  const { t } = useTranslation("generation");
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState(ALL_CATEGORIES);
  const deferredSearch = useDeferredValue(search);
  const categories = useMemo(
    () =>
      [
        ...new Set(
          presets
            .map((preset) => preset.category?.trim())
            .filter((value): value is string => Boolean(value)),
        ),
      ].toSorted((left, right) => left.localeCompare(right)),
    [presets],
  );
  const categoryOptions = useMemo(
    () => [
      { value: ALL_CATEGORIES, label: t("allPresetCategories") },
      ...categories.map((value) => ({ value, label: value })),
      { value: UNCATEGORIZED, label: t("uncategorizedPresets") },
    ],
    [categories, t],
  );
  const visiblePresets = useMemo(() => {
    const query = deferredSearch.trim().toLocaleLowerCase();
    return presets.filter((preset) => {
      const matchesCategory =
        category === ALL_CATEGORIES ||
        (category === UNCATEGORIZED ? !preset.category?.trim() : preset.category === category);
      if (!matchesCategory) return false;
      if (!query) return true;
      return [preset.name, preset.category, preset.description].some((value) =>
        value?.toLocaleLowerCase().includes(query),
      );
    });
  }, [category, deferredSearch, presets]);

  function close() {
    setSearch("");
    setCategory(ALL_CATEGORIES);
    onClose();
  }

  return (
    <AppModal open={open} title={title} onClose={close}>
      <div className="grid gap-3">
        <div className="grid grid-cols-[minmax(0,1fr)_9rem] items-center gap-2">
          <label className="flex min-w-0 items-center gap-2 border border-app-border bg-black/20 px-3 text-app-muted">
            <Search aria-hidden="true" className="size-4 shrink-0" />
            <input
              aria-label={t("searchPresets")}
              value={search}
              placeholder={t("searchPresets")}
              className="h-9 min-w-0 flex-1 bg-transparent text-sm text-app-text outline-none placeholder:text-app-muted"
              onChange={(event) => setSearch(event.target.value)}
            />
          </label>
          <AppSelect
            aria-label={t("filterPresetCategory")}
            value={category}
            options={categoryOptions}
            onValueChange={setCategory}
          />
        </div>

        {visiblePresets.length === 0 ? (
          <div className="grid min-h-48 place-items-center border border-dashed border-app-border text-sm text-app-muted">
            {presets.length === 0 ? t("noPresets") : t("noMatchingPresets")}
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
            {visiblePresets.map((preset) => {
              const selected = preset.preset_id === selectedPresetId;
              return (
                <button
                  key={preset.preset_id}
                  type="button"
                  aria-pressed={selected}
                  className={[
                    "group relative grid min-w-0 content-start gap-2 border bg-black/20 p-2 text-left",
                    selected
                      ? "border-brand-400/70 bg-brand-500/10"
                      : "border-app-border hover:border-brand-400/60 hover:bg-app-surface",
                  ].join(" ")}
                  onClick={() => {
                    onSelect(preset.preset_id);
                    close();
                  }}
                >
                  {selected ? (
                    <span className="absolute top-3 right-3 z-10 grid size-5 place-items-center bg-brand-500 text-white">
                      <Check aria-hidden="true" className="size-3.5" />
                    </span>
                  ) : null}
                  <GenerationResourceThumbnail
                    resource={preset.preview}
                    alt={preset.name}
                    className="aspect-square w-full"
                  />
                  <span className="min-w-0">
                    <span className="block truncate text-xs font-semibold text-app-text">
                      {preset.name}
                    </span>
                    <span className="mt-0.5 block truncate text-[11px] text-app-muted">
                      {preset.category ?? t("uncategorizedPresets")}
                    </span>
                    {preset.description ? (
                      <span className="mt-1 line-clamp-2 block text-[11px] text-app-muted/80">
                        {preset.description}
                      </span>
                    ) : null}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </AppModal>
  );
}
