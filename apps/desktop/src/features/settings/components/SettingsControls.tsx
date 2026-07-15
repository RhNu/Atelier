import { Loader2 } from "lucide-react";
import { useCallback, type ChangeEvent, type ReactNode } from "react";

import { AppSelect } from "@/components/ui";

import { parseNumberInput } from "../settings-utils";

export function SectionHeader({
  kicker,
  title,
  description,
  children,
}: {
  kicker: string;
  title: string;
  description: string;
  children?: ReactNode;
}) {
  return (
    <header className="flex min-h-16 flex-wrap items-center justify-between gap-3 border-b border-app-border px-4 py-3">
      <div>
        <p className="text-xs font-semibold text-brand-200 uppercase">{kicker}</p>
        <h2 className="mt-1 text-base font-semibold text-white">{title}</h2>
        <p className="mt-1 text-sm text-app-muted">{description}</p>
      </div>
      {children ? <div className="flex items-center gap-2">{children}</div> : null}
    </header>
  );
}

export function LoadingPanel({ label }: { label: string }) {
  return (
    <div className="flex h-full min-h-32 items-center justify-center text-sm text-app-muted">
      <Loader2 aria-hidden="true" className="mr-2 size-4 animate-spin" />
      {label}
    </div>
  );
}

export function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="border border-app-border bg-app-surface p-3">
      <dt className="text-xs text-app-muted uppercase">{label}</dt>
      <dd className="mt-1 truncate font-semibold text-app-text">{value}</dd>
    </div>
  );
}

export function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  disabled,
  autoComplete,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: "text" | "password";
  placeholder?: string;
  disabled?: boolean;
  autoComplete?: string;
}) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onChange(event.target.value);
    },
    [onChange],
  );

  return (
    <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        value={value}
        type={type}
        placeholder={placeholder}
        disabled={disabled}
        autoComplete={autoComplete}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400 disabled:cursor-not-allowed disabled:opacity-50"
        onChange={handleChange}
      />
    </label>
  );
}

export function NumberField({
  label,
  value,
  onChange,
  step = "1",
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  step?: string;
}) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onChange(parseNumberInput(event.target.value));
    },
    [onChange],
  );

  return (
    <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        type="number"
        value={String(value)}
        step={step}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
        onChange={handleChange}
      />
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
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      onChange(event.target.value);
    },
    [onChange],
  );

  return (
    <label className="grid gap-2 text-xs font-semibold text-app-muted uppercase">
      {label}
      <AppSelect aria-label={label} value={value} options={options} onChange={handleChange} />
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
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onChange(event.target.checked);
    },
    [onChange],
  );

  return (
    <label className="flex items-center gap-3 border border-app-border bg-app-surface px-3 py-2 text-sm text-app-text">
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        className="size-4 border border-app-border bg-black/20"
        onChange={handleChange}
      />
      {label}
    </label>
  );
}
