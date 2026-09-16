/* eslint-disable react/only-export-components, react-perf/jsx-no-new-function-as-prop */
import type { ChangeEvent, ReactNode } from "react";

import { useToastStore } from "@/stores/toast-store";
import type { SaveAgentConnectionRequestDto, SaveAgentModelRequestDto } from "@/types";

export const DEFAULT_CONTEXT_WINDOW = 32_768;
export const DEFAULT_MAX_OUTPUT = 4_096;
export const DEFAULT_TEMPERATURE = 0.3;

export function SettingsBlock({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="grid gap-4 border border-app-border bg-app-surface/45 p-4">
      <div>
        <h3 className="text-sm font-semibold text-white">{title}</h3>
        <p className="mt-1 text-xs leading-5 text-app-muted">{description}</p>
      </div>
      {children}
    </section>
  );
}

export function TextAreaField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => onChange(event.target.value);
  return (
    <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
      {label}
      <textarea
        aria-label={label}
        value={value}
        rows={4}
        className="resize-y border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
        onChange={handleChange}
      />
    </label>
  );
}

export function newConnectionDraft(): SaveAgentConnectionRequestDto {
  return {
    id: crypto.randomUUID(),
    display_name: "",
    base_url: "",
    protocol: "chat_completions",
    auth_kind: "bearer",
    secret: null,
  };
}

export function newModelDraft(connectionId: string): SaveAgentModelRequestDto {
  return {
    id: crypto.randomUUID(),
    connection_id: connectionId,
    wire_model_id: "",
    display_name: "",
    context_window: DEFAULT_CONTEXT_WINDOW,
    max_output_tokens: DEFAULT_MAX_OUTPUT,
    temperature: DEFAULT_TEMPERATURE,
    capabilities: { image_input: "none" },
  };
}

export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function notifyAgentSettingsError(
  push: ReturnType<typeof useToastStore.getState>["push"],
  title: string,
  error: unknown,
) {
  push({ level: "error", title, message: formatError(error) });
}
