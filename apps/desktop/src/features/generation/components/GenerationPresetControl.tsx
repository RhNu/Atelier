/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { LibraryBig, Trash2, WandSparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton } from "@/components/ui";
import { ResourcePickerDialog } from "@/features/resources";
import type { PromptPresetDto } from "@/types";

type GenerationPresetControlProps = {
  label: string;
  noPresetLabel: string;
  libraryTitle: string;
  namespace: "main_preset" | "character_preset";
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
  namespace,
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
  const pickerItems = useMemo(
    () =>
      presets.map((preset) => ({
        id: preset.preset_id,
        label: preset.display_name || preset.identifier,
        aliases: preset.aliases,
        description: preset.description,
        preview: preset.preview,
      })),
    [presets],
  );
  const displayName = pending
    ? t("loadingPresets")
    : (selectedPreset?.display_name ?? noPresetLabel);

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

      <ResourcePickerDialog
        open={dialogOpen}
        title={libraryTitle}
        namespace={namespace}
        items={pickerItems}
        selectedItemId={selectedPresetId}
        pending={pending}
        emptyLabel={t("noPresets")}
        noMatchesLabel={t("noMatchingPresets")}
        onClose={() => setDialogOpen(false)}
        onSelect={(item) => onSelect(item.id)}
      />
    </div>
  );
}
