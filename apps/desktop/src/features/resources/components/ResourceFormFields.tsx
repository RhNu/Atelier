/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { type ChangeEvent, useId } from "react";

import { AppSelect } from "@/components/ui";
import { NaiPromptEditor } from "@/features/prompt-editor";

export function TextInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

export function CategoryInput({
  label,
  value,
  suggestions,
  onChange,
}: {
  label: string;
  value: string;
  suggestions: ReadonlyArray<string>;
  onChange: (value: string) => void;
}) {
  const suggestionsId = useId();
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        list={suggestions.length > 0 ? suggestionsId : undefined}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
      {suggestions.length > 0 ? (
        <datalist id={suggestionsId}>
          {suggestions.map((suggestion) => (
            <option key={suggestion} value={suggestion}>
              {suggestion}
            </option>
          ))}
        </datalist>
      ) : null}
    </label>
  );
}

export function NumberInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        type="number"
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || 0)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </label>
  );
}

export function TextArea({
  label,
  value,
  minRows,
  onChange,
}: {
  label: string;
  value: string;
  minRows: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <textarea
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={[
          minRows,
          "resize-none border border-app-border bg-black/20 p-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400",
        ].join(" ")}
      />
    </label>
  );
}

export function PromptTextArea({
  label,
  value,
  minHeight = 160,
  onChange,
}: {
  label: string;
  value: string;
  minHeight?: number;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted">
      {label}
      <NaiPromptEditor
        aria-label={label}
        value={value}
        onChange={onChange}
        profile="novelai_v45"
        minHeight={minHeight}
      />
    </label>
  );
}

export function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex h-9 items-center gap-2 border border-app-border bg-black/20 px-3 text-sm text-app-text">
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  );
}

export function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: ReadonlyArray<{ value: string; label: string }>;
  onChange: (value: string) => void;
}) {
  const handleChange = (event: ChangeEvent<HTMLSelectElement>) => onChange(event.target.value);
  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <AppSelect aria-label={label} value={value} options={options} onChange={handleChange} />
    </label>
  );
}
