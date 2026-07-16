/* eslint-disable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { useCallback, useMemo, useState, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import type { GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";

type GenerationParamsPanelProps = {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  onPatchSize: (
    patch: Partial<GenerationDraft["size"]>,
    options?: GenerationDraftPatchOptions,
  ) => void;
  onFlush: () => void;
};

type SizePreset = {
  value: string;
  group: "normal" | "large" | "small";
  shape: "portrait" | "landscape" | "square";
  width: number;
  height: number;
};

const SIZE_PRESETS: ReadonlyArray<SizePreset> = [
  { value: "normal-portrait", group: "normal", shape: "portrait", width: 832, height: 1216 },
  { value: "normal-landscape", group: "normal", shape: "landscape", width: 1216, height: 832 },
  { value: "normal-square", group: "normal", shape: "square", width: 1024, height: 1024 },
  { value: "large-portrait", group: "large", shape: "portrait", width: 1024, height: 1536 },
  { value: "large-landscape", group: "large", shape: "landscape", width: 1536, height: 1024 },
  { value: "small-portrait", group: "small", shape: "portrait", width: 512, height: 768 },
  { value: "small-landscape", group: "small", shape: "landscape", width: 768, height: 512 },
  { value: "small-square", group: "small", shape: "square", width: 640, height: 640 },
];

export function GenerationParamsPanel({
  draft,
  onPatch,
  onPatchSize,
  onFlush,
}: GenerationParamsPanelProps) {
  const { t } = useTranslation("generation");
  const matchedPreset = useMemo(
    () =>
      SIZE_PRESETS.find(
        (preset) => preset.width === draft.size.width && preset.height === draft.size.height,
      )?.value ?? "custom",
    [draft.size.height, draft.size.width],
  );
  const [forceCustom, setForceCustom] = useState(false);
  const selectedPreset = forceCustom ? "custom" : matchedPreset;

  const handlePresetChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const preset = SIZE_PRESETS.find((item) => item.value === event.target.value);
      if (!preset) {
        setForceCustom(true);
        return;
      }
      setForceCustom(false);
      onPatchSize({ width: preset.width, height: preset.height }, { persist: "immediate" });
    },
    [onPatchSize],
  );

  return (
    <section className="space-y-4 border-b border-app-border p-4">
      <header>
        <h2 className="text-xs font-bold text-app-muted uppercase">{t("imageSettings")}</h2>
      </header>

      <div className="flex gap-2">
        <select
          aria-label={t("sizePreset")}
          value={selectedPreset}
          onChange={handlePresetChange}
          className="h-9 min-w-0 flex-1 border border-app-border bg-app-surface px-3 text-sm text-app-text outline-none focus:border-brand-400"
        >
          {(["normal", "large", "small"] as const).map((group) => (
            <optgroup key={group} label={t(`sizeGroup.${group}`)}>
              {SIZE_PRESETS.filter((preset) => preset.group === group).map((preset) => (
                <option key={preset.value} value={preset.value}>
                  {t(`sizeShape.${preset.shape}`)} ({preset.width}×{preset.height})
                </option>
              ))}
            </optgroup>
          ))}
          <optgroup label={t("custom")}>
            <option value="custom">{t("custom")}</option>
          </optgroup>
        </select>
        <div className="flex min-w-0 flex-1 items-center border border-app-border bg-black/20 px-2">
          <input
            aria-label={t("width")}
            type="number"
            min={64}
            max={1600}
            step={64}
            value={draft.size.width}
            className="min-w-0 flex-1 bg-transparent text-center text-sm outline-none"
            onChange={(event) => {
              setForceCustom(true);
              onPatchSize({ width: Number(event.target.value) });
            }}
            onBlur={onFlush}
          />
          <span className="text-app-muted">×</span>
          <input
            aria-label={t("height")}
            type="number"
            min={64}
            max={1600}
            step={64}
            value={draft.size.height}
            className="min-w-0 flex-1 bg-transparent text-center text-sm outline-none"
            onChange={(event) => {
              setForceCustom(true);
              onPatchSize({ height: Number(event.target.value) });
            }}
            onBlur={onFlush}
          />
        </div>
      </div>

      <CountSelector
        label={t("requests")}
        value={draft.requestCount}
        values={[1, 2, 3, 4, 5, 6, 7, 8]}
        onChange={(requestCount) => onPatch({ requestCount }, { persist: "immediate" })}
      />
      <CountSelector
        label={t("samplesPerRequest")}
        value={draft.nSamples}
        values={[1, 2, 3, 4]}
        onChange={(nSamples) => onPatch({ nSamples }, { persist: "immediate" })}
      />
    </section>
  );
}

function CountSelector({
  label,
  value,
  values,
  onChange,
}: {
  label: string;
  value: number;
  values: ReadonlyArray<number>;
  onChange: (value: number) => void;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-xs">
        <span className="font-semibold text-app-muted uppercase">{label}</span>
        <span className="font-semibold text-app-text">{value}</span>
      </div>
      <div
        className="grid overflow-hidden border border-app-border bg-black/20"
        style={{ gridTemplateColumns: `repeat(${values.length}, minmax(0, 1fr))` }}
      >
        {values.map((item) => (
          <button
            key={item}
            type="button"
            aria-label={`${label} ${item}`}
            aria-pressed={value === item}
            className={[
              "h-9 border-r border-app-border text-sm font-semibold last:border-r-0",
              value === item
                ? "bg-brand-500/20 text-brand-100"
                : "text-app-muted hover:bg-app-surface hover:text-app-text",
            ].join(" ")}
            onClick={() => onChange(item)}
          >
            {item}
          </button>
        ))}
      </div>
    </div>
  );
}
