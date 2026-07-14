import {
  Brush,
  Eraser,
  ImagePlus,
  Layers,
  Palette,
  Save,
  ScanLine,
  Sparkles,
  Trash2,
} from "lucide-react";
import type { ChangeEvent } from "react";

import {
  AppButton,
  AppPanel,
  AppSelect,
  AppTabs,
  EmptyState,
  ResourceImage,
} from "../../../components/ui";
import type { DirectorToolDto, DirectorToolResultDto } from "../../../types";
import { DIRECTOR_TOOLS, type DirectorInput } from "../director-model";

const TOOL_TABS = DIRECTOR_TOOLS.map((tool) => ({ value: tool.value, label: tool.label }));
const SAFETY_OPTIONS = [
  { value: "", label: "Clear override" },
  { value: "safe", label: "Safe" },
  { value: "sensitive", label: "Sensitive" },
  { value: "hidden", label: "Hidden" },
] as const;

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
  return (
    <AppPanel className="flex min-h-0 flex-col overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Input</h2>
      </header>
      <div className="grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)_auto] gap-3 p-3">
        {loadingImage ? (
          <EmptyState title="Loading image" />
        ) : imageError ? (
          <EmptyState title="Input unavailable" description={imageError} />
        ) : input ? (
          <div className="min-h-0 overflow-hidden border border-app-border bg-black/20">
            <ResourceImage src={imageSrc} alt="Director source" className="h-full w-full" />
          </div>
        ) : (
          <EmptyState title="No director input" description="Import or paste an image." />
        )}
        <div className="grid gap-2">
          {input ? <p className="truncate text-xs text-app-muted">{input.label}</p> : null}
          <div className="grid grid-cols-2 gap-2">
            <AppButton variant="secondary" onClick={onPick} disabled={pickPending}>
              <ImagePlus aria-hidden="true" className="size-4" />
              {input ? "Replace" : "Import"}
            </AppButton>
            <AppButton variant="secondary" onClick={onPaste}>
              <Layers aria-hidden="true" className="size-4" />
              Paste
            </AppButton>
          </div>
          <AppButton variant="ghost" onClick={onClear} disabled={!input}>
            <Trash2 aria-hidden="true" className="size-4" />
            Clear input
          </AppButton>
        </div>
      </div>
    </AppPanel>
  );
}

export function DirectorPreviewPanel({
  sourceSrc,
  resultSrc,
  resultPending,
  resultError,
}: {
  sourceSrc: string | null;
  resultSrc: string | null;
  resultPending: boolean;
  resultError: string | null;
}) {
  return (
    <AppPanel className="min-h-0 overflow-hidden bg-black/25">
      <div className="grid h-full min-h-0 grid-cols-2 gap-3 p-3">
        <PreviewFrame title="Original" src={sourceSrc} pending={false} error={null} />
        <PreviewFrame title="Result" src={resultSrc} pending={resultPending} error={resultError} />
      </div>
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
  return (
    <section className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-2">
      <h2 className="text-xs font-semibold text-app-muted uppercase">{title}</h2>
      {pending ? (
        <EmptyState title="Loading result" />
      ) : error ? (
        <EmptyState title="Image unavailable" description={error} />
      ) : (
        <ResourceImage src={src} fallbackLabel="No image" className="h-full w-full bg-app-bg" />
      )}
    </section>
  );
}

export function DirectorRunPanel({
  tool,
  toolDescription,
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
  onRun,
  onSave,
  onSafetyChange,
  onApplySafety,
}: {
  tool: DirectorToolDto;
  toolDescription: string;
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
  onDefryChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onRun: () => void;
  onSave: () => void;
  onSafetyChange: (event: ChangeEvent<HTMLSelectElement>) => void;
  onApplySafety: () => void;
}) {
  return (
    <AppPanel className="flex min-h-0 flex-col overflow-hidden">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Run State</h2>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        <div className="grid gap-4 text-sm text-app-text">
          {readinessPending ? (
            <p className="text-app-muted">Checking active NovelAI account</p>
          ) : null}
          {readinessError ? <p className="text-rose-100">{readinessError}</p> : null}
          <AppTabs value={tool} tabs={TOOL_TABS} onChange={onToolChange} label="Director tools" />
          <p className="text-sm text-app-muted">{toolDescription}</p>
          {showsPrompt ? (
            <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
              {promptRequired ? "Prompt required" : "Prompt optional"}
              <textarea
                aria-label="Director prompt"
                value={prompt}
                onChange={onPromptChange}
                className="min-h-24 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
              />
            </label>
          ) : null}
          {showsPrompt ? (
            <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
              Defry
              <input
                aria-label="Defry"
                type="number"
                value={defry}
                min={0}
                max={5}
                step={1}
                onChange={onDefryChange}
                className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
              />
            </label>
          ) : null}
          <AppButton onClick={onRun} disabled={!canRun}>
            {runIcon(tool)}
            {runPending ? "Running Director tool" : "Run Director tool"}
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
  return (
    <section className="grid gap-3 border border-app-border bg-black/20 p-3">
      <div>
        <p className="text-xs font-semibold text-app-muted uppercase">Result</p>
        <p className="mt-1 truncate text-sm text-app-text">{result.item_id}</p>
      </div>
      <AppButton variant="secondary" onClick={onSave} disabled={savePending}>
        <Save aria-hidden="true" className="size-4" />
        Save result
      </AppButton>
      <label
        htmlFor="director-safety-override"
        className="grid gap-1 text-xs font-semibold text-app-muted uppercase"
      >
        Safety override
        <AppSelect
          id="director-safety-override"
          aria-label="Director safety override"
          value={safetyOverride}
          options={SAFETY_OPTIONS}
          onChange={onSafetyChange}
        />
      </label>
      <AppButton variant="secondary" onClick={onApplySafety} disabled={safetyPending}>
        Apply safety override
      </AppButton>
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
