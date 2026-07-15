import { AppModal } from "../../../components/ui";
import type { CompiledGenerationPromptDto, CompiledPromptDto } from "../../../types";

export function GenerationPromptCompileDialog({
  open,
  pending,
  error,
  compiled,
  onClose,
}: {
  open: boolean;
  pending: boolean;
  error: string | null;
  compiled: CompiledGenerationPromptDto | null;
  onClose: () => void;
}) {
  return (
    <AppModal open={open} title="Compiled prompt preview" onClose={onClose}>
      {pending ? <p className="text-sm text-app-muted">Compiling prompt stack…</p> : null}
      {error ? <p className="text-sm text-rose-100">{error}</p> : null}
      {compiled ? (
        <div className="space-y-4">
          <CompiledPromptSection title="Positive prompt" prompt={compiled.prompt} />
          {compiled.negative_prompt ? (
            <CompiledPromptSection title="Undesired content" prompt={compiled.negative_prompt} />
          ) : null}
          {compiled.characters.map((character, index) => (
            <div key={`compiled-character-${index}`} className="space-y-2">
              <CompiledPromptSection title={`Character ${index + 1}`} prompt={character.prompt} />
              {character.negative_prompt ? (
                <CompiledPromptSection
                  title={`Character ${index + 1} undesired content`}
                  prompt={character.negative_prompt}
                />
              ) : null}
            </div>
          ))}
        </div>
      ) : null}
    </AppModal>
  );
}

function CompiledPromptSection({ title, prompt }: { title: string; prompt: CompiledPromptDto }) {
  return (
    <section className="border border-app-border bg-black/20 p-3">
      <h3 className="text-xs font-bold text-app-muted uppercase">{title}</h3>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {prompt.expanded_prompt || "Empty"}
      </p>
      {prompt.trace.function_calls.length ? (
        <details className="mt-3 border-t border-app-border pt-2">
          <summary className="cursor-pointer text-xs font-semibold text-brand-200">
            {prompt.trace.function_calls.length} function calls
          </summary>
          <div className="mt-2 space-y-2">
            {prompt.trace.function_calls.map((entry, index) => (
              <div
                key={`${entry.function_name}-${index}`}
                className="border border-app-border bg-app-surface/60 p-2 text-xs"
              >
                <p className="font-semibold text-app-text">{entry.raw_call}</p>
                <p className="mt-1 text-app-muted">
                  {entry.resolved_arguments.join("; ") || "No resolved arguments"}
                </p>
                {entry.result_text ? (
                  <p className="mt-1 text-app-text">{entry.result_text}</p>
                ) : null}
              </div>
            ))}
          </div>
        </details>
      ) : null}
    </section>
  );
}
