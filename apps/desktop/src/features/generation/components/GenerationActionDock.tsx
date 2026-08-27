/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { ChevronDown, ChevronUp, RotateCcw, WandSparkles } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppRangeField, AppSelect } from "@/components/ui";
import { OpusAllowanceMetric } from "@/features/account/components/OpusAllowanceMetric";
import type { ModelCapabilitiesDto, V5UsageStatusDto } from "@/types";

import type { GenerationDraft } from "../model/generation-draft";
import {
  generationImageFormatOptions,
  generationNoiseScheduleOptions,
  generationSamplerDisplayNames,
  generationSamplerSelectOptions,
  toImageFormat,
  toNoiseSchedule,
  toSampler,
  toSelectOptions,
} from "../model/generation-options";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { SeedInput } from "./SeedInput";

type GenerationActionDockProps = {
  draft: GenerationDraft;
  balance: number | null;
  balancePending: boolean;
  balanceError: string | null;
  estimate: number | null;
  perImageCost: number | null;
  estimatePending: boolean;
  estimateError: string | null;
  submitPending: boolean;
  validationError: string | null;
  submitError: string | null;
  draftLoadError: string | null;
  draftSaveError: string | null;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  onFlush: () => void;
  onSubmit: () => void;
  onResetParameters: () => void;
  onRetryDraftSave: () => void;
  onClearStoredDraft: () => void;
  capabilities?: ModelCapabilitiesDto;
  opusAllowance?: V5UsageStatusDto | null;
};

const NOISE_OPTIONS = toSelectOptions(generationNoiseScheduleOptions);

export function GenerationActionDock({
  draft,
  balance,
  balancePending,
  balanceError,
  estimate,
  perImageCost,
  estimatePending,
  estimateError,
  submitPending,
  validationError,
  submitError,
  draftLoadError,
  draftSaveError,
  onPatch,
  onFlush,
  onSubmit,
  onResetParameters,
  onRetryDraftSave,
  onClearStoredDraft,
  capabilities,
  opusAllowance,
}: GenerationActionDockProps) {
  const { t } = useTranslation("generation");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const totalImages = draft.requestCount * draft.nSamples;
  const insufficientBalance = balance !== null && estimate !== null && estimate > balance;
  const samplerLabel = generationSamplerDisplayNames[draft.sampler];
  const streamingUnsupported = capabilities?.supports_streaming === false;
  const showAllowanceWarning =
    opusAllowance !== null &&
    opusAllowance !== undefined &&
    (opusAllowance.is_negative || opusAllowance.percent <= 2) &&
    perImageCost !== null;
  const formatOptions = useMemo(
    () => [
      { value: "default", label: t("workspaceDefault") },
      ...toSelectOptions(generationImageFormatOptions),
    ],
    [t],
  );
  const toggleSettings = useCallback(() => setSettingsOpen((value) => !value), []);

  return (
    <div className="shrink-0 border-t border-app-border bg-app-panel shadow-[0_-8px_24px_rgba(0,0,0,0.28)]">
      {settingsOpen ? (
        <div className="max-h-[60vh] space-y-4 overflow-y-auto border-b border-app-border p-4">
          <div className="flex items-center justify-between gap-3">
            <h2 className="text-xs font-bold text-app-muted uppercase">{t("aiSettings")}</h2>
            <div className="flex items-center gap-1">
              <button
                type="button"
                className="grid size-8 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
                aria-label={t("resetParameters")}
                onClick={onResetParameters}
              >
                <RotateCcw aria-hidden="true" className="size-4" />
              </button>
              <button
                type="button"
                className="grid size-8 place-items-center text-app-muted hover:bg-app-surface hover:text-app-text"
                aria-label={t("collapseAiSettings")}
                onClick={toggleSettings}
              >
                <ChevronDown aria-hidden="true" className="size-4" />
              </button>
            </div>
          </div>

          <AppRangeField
            label={t("steps")}
            value={draft.steps}
            min={1}
            max={50}
            step={1}
            onChange={(steps) => onPatch({ steps })}
            onCommit={onFlush}
          />
          <AppRangeField
            label={t("scale")}
            value={draft.scale}
            min={0}
            max={10}
            step={0.1}
            action={
              capabilities?.supports_variety_boost ? (
                <button
                  type="button"
                  aria-pressed={draft.varietyBoost}
                  className={[
                    "border px-2 py-1 text-[11px] font-semibold",
                    draft.varietyBoost
                      ? "border-brand-400/60 bg-brand-500/20 text-brand-100"
                      : "border-app-border text-app-muted hover:text-app-text",
                  ].join(" ")}
                  onClick={() =>
                    onPatch({ varietyBoost: !draft.varietyBoost }, { persist: "immediate" })
                  }
                >
                  {t("varietyBoost")}
                </button>
              ) : undefined
            }
            onChange={(scale) => onPatch({ scale })}
            onCommit={onFlush}
          />

          <div className="grid grid-cols-2 gap-3">
            <SeedInput
              label={t("seed")}
              value={draft.seedMode === "random" ? 0 : draft.seed}
              randomPlaceholder={t("random")}
              onChange={(seed) => onPatch({ seedMode: seed === 0 ? "random" : "fixed", seed })}
              onBlur={onFlush}
            />
            <SelectControl
              label={t("sampler")}
              value={draft.sampler}
              options={generationSamplerSelectOptions}
              onChange={(sampler) => onPatch({ sampler: toSampler(sampler) })}
              onBlur={onFlush}
            />
          </div>

          <details className="group border-t border-app-border pt-3">
            <summary className="flex cursor-pointer list-none items-center justify-between text-xs font-semibold text-app-muted uppercase">
              {t("advanced")}
              <ChevronDown className="size-4 group-open:rotate-180" />
            </summary>
            <div className="mt-4 space-y-4">
              <AppRangeField
                label={t("cfgRescale")}
                value={draft.cfgRescale}
                min={0}
                max={1}
                step={0.01}
                onChange={(cfgRescale) => onPatch({ cfgRescale })}
                onCommit={onFlush}
              />
              <SelectControl
                label={t("noiseSchedule")}
                value={draft.noiseSchedule}
                options={NOISE_OPTIONS}
                onChange={(noiseSchedule) =>
                  onPatch({
                    noiseSchedule: toNoiseSchedule(noiseSchedule),
                  })
                }
                onBlur={onFlush}
              />
              <ToggleControl
                label={t("strictMode")}
                checked={draft.strictMode}
                onChange={(strictMode) => onPatch({ strictMode }, { persist: "immediate" })}
              />
            </div>
          </details>

          <details className="group border-t border-app-border pt-3">
            <summary className="flex cursor-pointer list-none items-center justify-between text-xs font-semibold text-app-muted uppercase">
              {t("software")}
              <ChevronDown className="size-4 group-open:rotate-180" />
            </summary>
            <div className="mt-4 space-y-4">
              <ToggleControl
                label={t("streamingPreview")}
                checked={draft.streamEnabled}
                disabled={streamingUnsupported}
                title={streamingUnsupported ? t("streamingPreviewUnsupported") : undefined}
                onChange={(streamEnabled) => onPatch({ streamEnabled }, { persist: "immediate" })}
              />
              <SelectControl
                label={t("outputFormat")}
                value={draft.imageFormat ?? "default"}
                options={formatOptions}
                onChange={(imageFormat) =>
                  onPatch({
                    imageFormat: imageFormat === "default" ? null : toImageFormat(imageFormat),
                  })
                }
                onBlur={onFlush}
              />
            </div>
          </details>
        </div>
      ) : null}

      <div className="space-y-2.5 p-3">
        <div className="space-y-1 border border-app-border bg-black/20 px-2.5 py-2 text-xs">
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0">
              <span className="text-app-muted">{t("balance")} </span>
              <strong className="text-app-text">
                {balancePending ? t("loading") : balance === null ? "—" : `${balance} Anlas`}
              </strong>
              {opusAllowance ? <OpusAllowanceMetric usage={opusAllowance} /> : null}
            </div>
          </div>
          {balanceError ? <p className="text-amber-200">{balanceError}</p> : null}
          {showAllowanceWarning ? (
            <p className="text-amber-200">{t("opusAllowanceLowWarning", { cost: perImageCost })}</p>
          ) : null}
          {estimateError ? <p className="text-amber-200">{estimateError}</p> : null}
          {insufficientBalance ? (
            <p className="text-amber-200">{t("insufficientBalance")}</p>
          ) : null}
        </div>

        {!settingsOpen ? (
          <button
            type="button"
            className="grid w-full grid-cols-[auto_auto_auto_1fr] gap-3 border border-transparent p-2 text-left text-xs hover:border-app-border hover:bg-app-surface/50"
            onClick={toggleSettings}
          >
            <SummaryValue label={t("steps")} value={String(draft.steps)} />
            <SummaryValue label={t("scale")} value={String(draft.scale)} />
            <SummaryValue
              label={t("seed")}
              value={draft.seedMode === "random" ? t("random") : String(draft.seed)}
            />
            <div className="flex min-w-0 items-center justify-between gap-2">
              <SummaryValue label={t("sampler")} value={samplerLabel} truncate />
              <ChevronUp aria-hidden="true" className="size-4 shrink-0 text-app-muted" />
            </div>
          </button>
        ) : null}

        {draftLoadError ? (
          <ErrorLine
            message={draftLoadError}
            action={t("clearSavedDraft")}
            onAction={onClearStoredDraft}
          />
        ) : null}
        {draftSaveError ? (
          <ErrorLine message={draftSaveError} action={t("retrySave")} onAction={onRetryDraftSave} />
        ) : null}
        {validationError ? <p className="text-sm text-amber-200">{validationError}</p> : null}
        {submitError ? <p className="text-sm text-rose-100">{submitError}</p> : null}

        <button
          type="button"
          className="flex h-12 w-full items-center justify-between bg-amber-700 px-4 font-bold text-white transition-colors hover:bg-amber-600 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={submitPending}
          onClick={onSubmit}
        >
          <span className="inline-flex items-center gap-2">
            <WandSparkles aria-hidden="true" className="size-4" />
            {submitPending ? t("queueingGeneration") : t("generateImages", { count: totalImages })}
          </span>
          <span className="bg-white/15 px-2 py-1 text-sm">
            {estimatePending ? "…" : (estimate ?? 0)} Anlas
          </span>
        </button>
      </div>
    </div>
  );
}

function SelectControl({
  label,
  value,
  options,
  onChange,
  onBlur,
}: {
  label: string;
  value: string;
  options: ReadonlyArray<{ value: string; label: string }>;
  onChange: (value: string) => void;
  onBlur: () => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <AppSelect
        aria-label={label}
        value={value}
        options={options}
        onValueChange={onChange}
        onBlur={onBlur}
      />
    </label>
  );
}

function ToggleControl({
  label,
  checked,
  disabled = false,
  title,
  onChange,
}: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  title?: string;
  onChange: (value: boolean) => void;
}) {
  return (
    <label
      className={[
        "flex items-center justify-between border border-app-border bg-black/20 px-3 py-2 text-sm text-app-text",
        disabled ? "cursor-not-allowed opacity-50" : "",
      ].join(" ")}
      title={title}
    >
      {label}
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function SummaryValue({
  label,
  value,
  truncate = false,
}: {
  label: string;
  value: string;
  truncate?: boolean;
}) {
  return (
    <div className="min-w-0">
      <p className="text-app-muted">{label}</p>
      <p
        className={["mt-0.5 font-semibold text-app-text", truncate ? "truncate" : ""].join(" ")}
        title={value}
      >
        {value}
      </p>
    </div>
  );
}

function ErrorLine({
  message,
  action,
  onAction,
}: {
  message: string;
  action: string;
  onAction: () => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2 border border-rose-500/40 bg-rose-950/30 p-2 text-xs text-rose-100">
      <p className="min-w-0 flex-1 truncate" title={message}>
        {message}
      </p>
      <AppButton variant="ghost" className="h-7 shrink-0 px-2 text-xs" onClick={onAction}>
        {action}
      </AppButton>
    </div>
  );
}
