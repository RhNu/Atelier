/* eslint-disable max-lines */
import {
  Brush,
  ChevronDown,
  Eraser,
  ImagePlus,
  Layers,
  Palette,
  RotateCw,
  Save,
  ScanLine,
  Sparkles,
  Trash2,
} from "lucide-react";
import { useCallback, useMemo, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import {
  AppButton,
  AppIconButton,
  AppPanel,
  AppRangeField,
  AppSelect,
  EmptyState,
  ResourceImage,
} from "@/components/ui";
import type { DirectorToolDto, DirectorToolResultDto } from "@/types";

import { DIRECTOR_TOOLS, type DirectorInput } from "../director-model";
export function DirectorInputPanel({
  input,
  imageSrc,
  loadingImage,
  imageError,
  pickPending,
  onPick,
  onPaste,
  onClear,
}: {
  input: DirectorInput | null;
  imageSrc: string | null;
  loadingImage: boolean;
  imageError: string | null;
  pickPending: boolean;
  onPick: () => void;
  onPaste: () => void;
  onClear: () => void;
}) {
  const { t } = useTranslation("director");
  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">{t("input")}</h2>
      </header>
      <div className="grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)_auto] gap-3 p-3">
        {loadingImage ? (
          <EmptyState title={t("loadingImage")} />
        ) : imageError ? (
          <EmptyState title={t("inputUnavailable")} description={imageError} />
        ) : input ? (
          <div className="min-h-0 overflow-hidden border border-app-border bg-black/20">
            <ResourceImage src={imageSrc} alt={t("sourceImage")} className="h-full w-full" />
          </div>
        ) : (
          <EmptyState title={t("noInput")} iconOnly />
        )}
        <div className="grid gap-2">
          {input ? <p className="truncate text-xs text-app-muted">{input.label}</p> : null}
          <div className="grid grid-cols-2 gap-2">
            <AppButton variant="secondary" onClick={onPick} disabled={pickPending}>
              <ImagePlus aria-hidden="true" className="size-4" />
              {input ? t("replace") : t("import")}
            </AppButton>
            <AppButton variant="secondary" onClick={onPaste}>
              <Layers aria-hidden="true" className="size-4" />
              {t("paste")}
            </AppButton>
          </div>
          <AppButton variant="ghost" onClick={onClear} disabled={!input}>
            <Trash2 aria-hidden="true" className="size-4" />
            {t("clearInput")}
          </AppButton>
        </div>
      </div>
    </AppPanel>
  );
}

export function DirectorPreviewPanel({
  resultSrc,
  resultPending,
  resultError,
}: {
  resultSrc: string | null;
  resultPending: boolean;
  resultError: string | null;
}) {
  const { t } = useTranslation("director");
  return (
    <AppPanel variant="section" className="min-h-0 overflow-hidden bg-black/25">
      <PreviewFrame
        title={t("output")}
        src={resultSrc}
        pending={resultPending}
        error={resultError}
      />
    </AppPanel>
  );
}

function PreviewFrame({
  title,
  src,
  pending,
  error,
}: {
  title: string;
  src: string | null;
  pending: boolean;
  error: string | null;
}) {
  const { t } = useTranslation("director");
  return (
    <section className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-2 p-3">
      <h2 className="text-xs font-semibold text-app-muted uppercase">{title}</h2>
      {pending ? (
        <EmptyState title={t("loadingResult")} />
      ) : error ? (
        <EmptyState title={t("imageUnavailable")} description={error} />
      ) : (
        <ResourceImage src={src} fallbackLabel={t("noImage")} className="h-full w-full bg-app-bg" />
      )}
    </section>
  );
}

export function DirectorRunPanel({
  tool,
  anlas,
  showsPrompt,
  promptRequired,
  prompt,
  defry,
  canRun,
  runPending,
  result,
  safetyOverride,
  readinessPending,
  readinessError,
  savePending,
  safetyPending,
  onToolChange,
  onPromptChange,
  onDefryChange,
  onRefresh,
  onRun,
  onSave,
  onSafetyChange,
  onApplySafety,
}: {
  tool: DirectorToolDto;
  anlas: number | null;
  showsPrompt: boolean;
  promptRequired: boolean;
  prompt: string;
  defry: number;
  canRun: boolean;
  runPending: boolean;
  result: DirectorToolResultDto | null;
  safetyOverride: string;
  readinessPending: boolean;
  readinessError: string | null;
  savePending: boolean;
  safetyPending: boolean;
  onToolChange: (value: string) => void;
  onPromptChange: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  onDefryChange: (value: number) => void;
  onRefresh: () => void;
  onRun: () => void;
  onSave: () => void;
  onSafetyChange: (event: ChangeEvent<HTMLSelectElement>) => void;
  onApplySafety: () => void;
}) {
  const { t } = useTranslation("director");
  return (
    <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex min-h-12 items-center justify-end gap-1 border-b border-app-border px-3 py-2">
        <span className="text-xs font-semibold text-app-text tabular-nums">
          {readinessPending && anlas === null ? "…" : anlas === null ? "—" : `${anlas} Anlas`}
        </span>
        <AppIconButton
          icon={RotateCw}
          label={t("refreshAnlas")}
          size="sm"
          disabled={readinessPending}
          className={readinessPending ? "[&>svg]:animate-spin" : ""}
          onClick={onRefresh}
        />
      </header>
      <div className="min-h-0 min-w-0 flex-1 overflow-auto p-3">
        <div className="grid min-w-0 gap-3 text-sm text-app-text">
          {readinessError ? <p className="break-words text-rose-100">{readinessError}</p> : null}
          <DirectorToolPicker tool={tool} onToolChange={onToolChange} />
          <p className="min-h-5 text-xs leading-5 text-app-muted">
            {t(`tool.${tool}.description`)}
          </p>
          {showsPrompt ? (
            <label className="grid min-w-0 gap-2 text-xs font-semibold text-app-muted uppercase">
              {promptRequired ? t("promptRequired") : t("promptOptional")}
              <textarea
                aria-label={t("prompt")}
                value={prompt}
                onChange={onPromptChange}
                className="min-h-20 min-w-0 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
              />
            </label>
          ) : null}
          {showsPrompt ? (
            <AppRangeField
              label={t("defry")}
              value={defry}
              min={0}
              max={5}
              step={1}
              onChange={onDefryChange}
            />
          ) : null}
          <AppButton className="w-full min-w-0" onClick={onRun} disabled={!canRun}>
            {runIcon(tool)}
            {runPending ? t("runningTool") : t("runTool")}
          </AppButton>
          {result ? (
            <ResultActions
              result={result}
              safetyOverride={safetyOverride}
              savePending={savePending}
              safetyPending={safetyPending}
              onSave={onSave}
              onSafetyChange={onSafetyChange}
              onApplySafety={onApplySafety}
            />
          ) : null}
        </div>
      </div>
    </AppPanel>
  );
}

function DirectorToolPicker({
  tool,
  onToolChange,
}: {
  tool: DirectorToolDto;
  onToolChange: (value: string) => void;
}) {
  const { t } = useTranslation("director");
  const handleToolChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onToolChange(event.target.value),
    [onToolChange],
  );
  return (
    <div role="radiogroup" aria-label={t("tools")} className="grid min-w-0 grid-cols-2 gap-1.5">
      {DIRECTOR_TOOLS.map((item) => (
        <label
          key={item.value}
          className={[
            "flex h-9 min-w-0 cursor-pointer items-center gap-2 border px-2 text-xs font-semibold transition-colors",
            tool === item.value
              ? "border-brand-400/70 bg-brand-500/15 text-white"
              : "border-app-border bg-black/20 text-app-muted hover:bg-app-surface hover:text-app-text",
          ].join(" ")}
        >
          <input
            type="radio"
            name="director-tool"
            value={item.value}
            checked={tool === item.value}
            aria-label={t(`tool.${item.value}.label`)}
            className="sr-only"
            onChange={handleToolChange}
          />
          {runIcon(item.value)}
          <span className="truncate">{t(`tool.${item.value}.label`)}</span>
        </label>
      ))}
    </div>
  );
}

function ResultActions({
  result,
  safetyOverride,
  savePending,
  safetyPending,
  onSave,
  onSafetyChange,
  onApplySafety,
}: {
  result: DirectorToolResultDto;
  safetyOverride: string;
  savePending: boolean;
  safetyPending: boolean;
  onSave: () => void;
  onSafetyChange: (event: ChangeEvent<HTMLSelectElement>) => void;
  onApplySafety: () => void;
}) {
  const { t } = useTranslation("director");
  const safetyOptions = useMemo(
    () => [
      { value: "", label: t("clearOverride") },
      { value: "safe", label: t("safe") },
      { value: "sensitive", label: t("sensitive") },
      { value: "hidden", label: t("hidden") },
    ],
    [t],
  );
  return (
    <section className="grid gap-3 border border-app-border bg-black/20 p-3">
      <div>
        <p className="text-xs font-semibold text-app-muted uppercase">{t("result")}</p>
        <p className="mt-1 truncate text-sm text-app-text">{result.item_id}</p>
      </div>
      <AppButton variant="secondary" onClick={onSave} disabled={savePending}>
        <Save aria-hidden="true" className="size-4" />
        {t("saveResult")}
      </AppButton>
      <details className="group border-t border-app-border pt-3">
        <summary className="flex cursor-pointer list-none items-center justify-between text-xs font-semibold text-app-muted uppercase">
          {t("advancedSettings")}
          <ChevronDown
            aria-hidden="true"
            className="size-4 transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="mt-3 grid gap-2">
          <label
            htmlFor="director-safety-override"
            className="grid gap-1 text-xs font-semibold text-app-muted uppercase"
          >
            {t("safetyOverride")}
            <AppSelect
              id="director-safety-override"
              aria-label={t("safetyOverride")}
              value={safetyOverride}
              options={safetyOptions}
              onChange={onSafetyChange}
            />
          </label>
          <AppButton variant="secondary" onClick={onApplySafety} disabled={safetyPending}>
            {t("applySafetyOverride")}
          </AppButton>
        </div>
      </details>
    </section>
  );
}

function runIcon(tool: DirectorToolDto) {
  const Icon =
    tool === "colorize"
      ? Palette
      : tool === "bg_removal"
        ? Eraser
        : tool === "sketch"
          ? Brush
          : tool === "lineart"
            ? ScanLine
            : Sparkles;
  return <Icon aria-hidden="true" className="size-4" />;
}
