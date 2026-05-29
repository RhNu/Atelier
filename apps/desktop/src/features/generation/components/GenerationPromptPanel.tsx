import { Eye, WandSparkles } from "lucide-react";
import { useCallback, type ChangeEvent } from "react";

import { AppButton, AppPanel } from "../../../components/ui";
import type { CompiledGenerationPromptDto, CompiledPromptDto } from "../../../types";
import type { GenerationDraft } from "../model/generation-draft";

type GenerationPromptPanelProps = {
  draft: GenerationDraft;
  submitError: string | null;
  validationError: string | null;
  compileError: string | null;
  compilePending: boolean;
  submitPending: boolean;
  compiledPreview: CompiledGenerationPromptDto | null;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  onSubmit: () => void;
  onCompile: () => void;
};

export function GenerationPromptPanel({
  draft,
  submitError,
  validationError,
  compileError,
  compilePending,
  submitPending,
  compiledPreview,
  onPatch,
  onSubmit,
  onCompile,
}: GenerationPromptPanelProps) {
  const handlePromptChange = useCallback(
    (event: ChangeEvent<HTMLTextAreaElement>) => {
      onPatch({ prompt: event.target.value });
    },
    [onPatch],
  );
  const handleNegativePromptChange = useCallback(
    (event: ChangeEvent<HTMLTextAreaElement>) => {
      onPatch({ negativePrompt: event.target.value });
    },
    [onPatch],
  );

  return (
    <AppPanel className="flex min-h-0 flex-col overflow-hidden">
      <header className="flex items-center justify-between gap-3 border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Prompt Stack</h2>
        <AppButton variant="ghost" onClick={onCompile} disabled={compilePending}>
          <Eye aria-hidden="true" className="size-4" />
          Compile prompt preview
        </AppButton>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto p-3">
        <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
          Positive prompt
          <textarea
            aria-label="Positive prompt"
            value={draft.prompt}
            onChange={handlePromptChange}
            className="min-h-44 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
          />
        </label>
        <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
          Undesired content
          <textarea
            aria-label="Undesired content"
            value={draft.negativePrompt}
            onChange={handleNegativePromptChange}
            className="min-h-24 resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
          />
        </label>
        {validationError ? <p className="text-sm text-amber-200">{validationError}</p> : null}
        {submitError ? <p className="text-sm text-rose-100">{submitError}</p> : null}
        {compileError ? <p className="text-sm text-rose-100">{compileError}</p> : null}
        <CompiledPromptPreview title="Positive preview" preview={compiledPreview?.prompt ?? null} />
        <CompiledPromptPreview
          title="Negative preview"
          preview={compiledPreview?.negative_prompt ?? null}
        />
        {compiledPreview?.characters.map((character, index) => (
          <CompiledPromptPreview
            key={`character-${index}`}
            title={`Character ${index + 1} preview`}
            preview={character.prompt}
          />
        ))}
      </div>
      <footer className="border-t border-app-border p-3">
        <AppButton className="w-full" onClick={onSubmit} disabled={submitPending}>
          <WandSparkles aria-hidden="true" className="size-4" />
          {submitPending ? "Queueing generation" : "Queue generation"}
        </AppButton>
      </footer>
    </AppPanel>
  );
}

function CompiledPromptPreview({
  title,
  preview,
}: {
  title: string;
  preview: CompiledPromptDto | null;
}) {
  if (!preview) {
    return null;
  }

  return (
    <article className="border border-app-border bg-app-surface/70 p-3">
      <h3 className="text-xs font-semibold text-app-muted uppercase">{title}</h3>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {preview.expanded_prompt || "Empty"}
      </p>
      <p className="mt-2 text-xs text-app-muted">
        {preview.trace.function_calls.length} function calls
      </p>
    </article>
  );
}
