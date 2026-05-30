/* eslint-disable max-lines, max-lines-per-function */
import {
  Brush,
  Clapperboard,
  Eraser,
  ImagePlus,
  Layers,
  Palette,
  Save,
  ScanLine,
  Sparkles,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ChangeEvent } from "react";

import {
  AppButton,
  AppPanel,
  AppSelect,
  AppTabs,
  AppToolbar,
  EmptyState,
  ResourceImage,
} from "../../components/ui";
import type {
  DirectorToolDto,
  DirectorToolResultDto,
  GallerySafetyOverrideDto,
  ImageInputDto,
  ResourceRefDto,
} from "../../types";
import { formatError } from "../gallery/gallery-utils";
import {
  useDirectorImageQuery,
  usePickDirectorImageMutation,
  useRunDirectorToolMutation,
  useSaveDirectorImageMutation,
  useSetDirectorSafetyOverrideMutation,
} from "./data/useDirectorActions";
import { useDirectorReadinessQuery } from "./data/useDirectorReadinessQuery";
import { useDirectorHandoffStore } from "./state/director-handoff-store";

type DirectorInput =
  | { kind: "resource"; resource: ResourceRefDto; label: string }
  | { kind: "inline"; imageBase64: string; src: string; label: string };

const DIRECTOR_TOOLS: ReadonlyArray<{
  value: DirectorToolDto;
  label: string;
  description: string;
}> = [
  { value: "lineart", label: "Lineart", description: "Extract clean line art" },
  { value: "sketch", label: "Sketch", description: "Create a loose sketch pass" },
  { value: "bg_removal", label: "Background", description: "Remove the background" },
  { value: "declutter", label: "Declutter", description: "Clean visual noise" },
  { value: "colorize", label: "Colorize", description: "Add color with an optional prompt" },
  { value: "emotion", label: "Emotion", description: "Change expression from a prompt" },
];

const TOOL_TABS = DIRECTOR_TOOLS.map((tool) => ({ value: tool.value, label: tool.label }));
const SAFETY_OPTIONS = [
  { value: "", label: "Clear override" },
  { value: "safe", label: "Safe" },
  { value: "sensitive", label: "Sensitive" },
  { value: "hidden", label: "Hidden" },
] as const;

export function DirectorPage() {
  const readinessQuery = useDirectorReadinessQuery();
  const pickInputMutation = usePickDirectorImageMutation();
  const runToolMutation = useRunDirectorToolMutation();
  const saveImageMutation = useSaveDirectorImageMutation();
  const safetyMutation = useSetDirectorSafetyOverrideMutation();
  const consumeHandoff = useDirectorHandoffStore((state) => state.consumePendingInput);

  const [input, setInput] = useState<DirectorInput | null>(null);
  const [tool, setTool] = useState<DirectorToolDto>("lineart");
  const [prompt, setPrompt] = useState("");
  const [defry, setDefry] = useState(0);
  const [result, setResult] = useState<DirectorToolResultDto | null>(null);
  const [safetyOverride, setSafetyOverride] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);

  const sourceResource = input?.kind === "resource" ? input.resource : null;
  const sourceImageQuery = useDirectorImageQuery(sourceResource);
  const resultImageQuery = useDirectorImageQuery(result?.resource ?? null);
  const sourceImageSrc = input?.kind === "inline" ? input.src : (sourceImageQuery.data ?? null);
  const resultImageSrc = resultImageQuery.data ?? null;
  const selectedTool = DIRECTOR_TOOLS.find((item) => item.value === tool) ?? DIRECTOR_TOOLS[0];
  const showsPrompt = tool === "colorize" || tool === "emotion";
  const promptRequired = tool === "emotion";
  const canRun =
    Boolean(input) && !runToolMutation.isPending && (!promptRequired || prompt.trim().length > 0);

  useEffect(() => {
    const resource = consumeHandoff();
    if (resource) {
      setInput({ kind: "resource", resource, label: resource.id });
      setResult(null);
      setActionError(null);
    }
  }, [consumeHandoff]);

  useEffect(() => {
    setSafetyOverride(result?.item.manual_safety_override ?? "");
  }, [result?.item.item_id, result?.item.manual_safety_override]);

  const runSummary = useMemo(
    () => ({
      tier: readinessQuery.data?.tier_name ?? "Unknown",
      anlas: readinessQuery.data?.anlas_balance ?? null,
      tool: selectedTool.description,
    }),
    [readinessQuery.data?.anlas_balance, readinessQuery.data?.tier_name, selectedTool.description],
  );

  const handlePickInput = useCallback(() => {
    setActionError(null);
    void pickInputMutation
      .mutateAsync()
      .then((resource) => {
        if (resource) {
          setInput({ kind: "resource", resource, label: resource.id });
          setResult(null);
        }
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [pickInputMutation]);

  const handlePasteInput = useCallback(() => {
    setActionError(null);
    void readClipboardImage()
      .then((pasted) => {
        setInput({
          kind: "inline",
          imageBase64: pasted.imageBase64,
          src: pasted.src,
          label: "Clipboard image",
        });
        setResult(null);
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, []);

  const handleClearInput = useCallback(() => {
    setInput(null);
    setResult(null);
    setActionError(null);
  }, []);

  const handleToolChange = useCallback((value: string) => {
    setTool(parseDirectorTool(value));
    setResult(null);
  }, []);

  const handlePromptChange = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
    setPrompt(event.target.value);
  }, []);

  const handleDefryChange = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    setDefry(clampDefry(Number(event.target.value)));
  }, []);

  const handleRunTool = useCallback(() => {
    if (!input || !canRun) {
      return;
    }
    setActionError(null);
    void runToolMutation
      .mutateAsync(buildDirectorRunRequest(input, tool, prompt, defry))
      .then(setResult)
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [canRun, defry, input, prompt, runToolMutation, tool]);

  const handleSaveResult = useCallback(() => {
    if (!result) {
      return;
    }
    setActionError(null);
    void saveImageMutation
      .mutateAsync({
        resource: result.resource,
        suggested_file_name: `${result.item_id}-${tool}`,
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [result, saveImageMutation, tool]);

  const handleSafetyChange = useCallback((event: ChangeEvent<HTMLSelectElement>) => {
    setSafetyOverride(event.target.value);
  }, []);

  const handleApplySafety = useCallback(() => {
    if (!result) {
      return;
    }
    setActionError(null);
    void safetyMutation
      .mutateAsync({
        item_id: result.item_id,
        manual_safety_override: parseSafetyOverride(safetyOverride),
      })
      .then((item) => setResult({ ...result, item }))
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [result, safetyMutation, safetyOverride]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Director</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Director Tools</h1>
        </div>
        <div className="flex items-center gap-2 text-xs text-app-muted">
          <Clapperboard aria-hidden="true" className="size-4" />
          <span>{runSummary.tier}</span>
          {runSummary.anlas === null ? null : <span>{runSummary.anlas} Anlas</span>}
        </div>
      </AppToolbar>

      {actionError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {actionError}
        </p>
      ) : null}

      <div className="grid min-h-0 flex-1 grid-cols-[340px_minmax(0,1fr)_340px] gap-3 p-3">
        <DirectorInputPanel
          input={input}
          imageSrc={sourceImageSrc}
          loadingImage={sourceImageQuery.isPending && Boolean(sourceResource)}
          imageError={sourceImageQuery.isError ? formatError(sourceImageQuery.error) : null}
          pickPending={pickInputMutation.isPending}
          onPick={handlePickInput}
          onPaste={handlePasteInput}
          onClear={handleClearInput}
        />

        <DirectorPreviewPanel
          sourceSrc={sourceImageSrc}
          resultSrc={resultImageSrc}
          resultPending={resultImageQuery.isPending && Boolean(result)}
          resultError={resultImageQuery.isError ? formatError(resultImageQuery.error) : null}
        />

        <DirectorRunPanel
          tool={tool}
          toolDescription={runSummary.tool}
          showsPrompt={showsPrompt}
          promptRequired={promptRequired}
          prompt={prompt}
          defry={defry}
          canRun={canRun}
          runPending={runToolMutation.isPending}
          result={result}
          safetyOverride={safetyOverride}
          readinessPending={readinessQuery.isPending}
          readinessError={readinessQuery.isError ? formatError(readinessQuery.error) : null}
          savePending={saveImageMutation.isPending}
          safetyPending={safetyMutation.isPending}
          onToolChange={handleToolChange}
          onPromptChange={handlePromptChange}
          onDefryChange={handleDefryChange}
          onRun={handleRunTool}
          onSave={handleSaveResult}
          onSafetyChange={handleSafetyChange}
          onApplySafety={handleApplySafety}
        />
      </div>
    </div>
  );
}

function DirectorInputPanel({
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

function DirectorPreviewPanel({
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

function DirectorRunPanel({
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

function buildDirectorRunRequest(
  input: DirectorInput,
  tool: DirectorToolDto,
  prompt: string,
  defry: number,
) {
  const image: ImageInputDto =
    input.kind === "resource"
      ? { kind: "resource_ref", resource: input.resource }
      : { kind: "inline_base64", image_base64: input.imageBase64 };
  const supportsPrompt = tool === "colorize" || tool === "emotion";
  return {
    run_id: `director-${createId()}`,
    tool,
    image,
    prompt: supportsPrompt && prompt.trim().length > 0 ? prompt.trim() : null,
    defry: supportsPrompt ? clampDefry(defry) : null,
    strict_mode: true,
  };
}

function parseDirectorTool(value: string): DirectorToolDto {
  return DIRECTOR_TOOLS.find((tool) => tool.value === value)?.value ?? "lineart";
}

function parseSafetyOverride(value: string): GallerySafetyOverrideDto | null {
  return value === "safe" || value === "sensitive" || value === "hidden" ? value : null;
}

function clampDefry(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.min(5, Math.floor(value))) : 0;
}

function createId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

async function readClipboardImage(): Promise<{ imageBase64: string; src: string }> {
  const clipboard = globalThis.navigator?.clipboard;
  if (!clipboard || !("read" in clipboard)) {
    throw new Error("Clipboard images are unavailable in this environment");
  }
  const items = await clipboard.read();
  for (const item of items) {
    const mimeType = item.types.find((type) => type.startsWith("image/"));
    if (mimeType) {
      const blob = await item.getType(mimeType);
      const imageBase64 = await blobToBase64(blob);
      return { imageBase64, src: `data:${mimeType};base64,${imageBase64}` };
    }
  }
  throw new Error("Clipboard does not contain an image");
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      const result = typeof reader.result === "string" ? reader.result : "";
      resolve(result.split(",")[1] ?? "");
    });
    reader.addEventListener("error", () => reject(new Error("Unable to read pasted image")));
    reader.readAsDataURL(blob);
  });
}
