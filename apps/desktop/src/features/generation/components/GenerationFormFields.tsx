import { useCallback, useId, type ChangeEvent } from "react";

import { AppSelect } from "../../../components/ui";

export type SelectOption = {
  value: string;
  label: string;
};

export function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: ReadonlyArray<SelectOption>;
  onChange: (value: string) => void;
}) {
  const id = useId();
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      onChange(event.target.value);
    },
    [onChange],
  );

  return (
    <div className="grid gap-1">
      <label htmlFor={id} className="text-xs font-semibold text-app-muted uppercase">
        {label}
      </label>
      <AppSelect id={id} value={value} options={options} onChange={handleChange} />
    </div>
  );
}

export function NumberField({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (value: number) => void;
}) {
  const id = useId();
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const parsed = Number(event.target.value);
      onChange(Number.isFinite(parsed) ? parsed : 0);
    },
    [onChange],
  );

  return (
    <div className="grid gap-1">
      <label htmlFor={id} className="text-xs font-semibold text-app-muted uppercase">
        {label}
      </label>
      <input
        id={id}
        aria-label={label}
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={handleChange}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
      />
    </div>
  );
}

export function BooleanField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  const id = useId();
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onChange(event.target.checked);
    },
    [onChange],
  );

  return (
    <label
      htmlFor={id}
      className="flex items-center gap-2 border border-app-border bg-app-surface/70 px-3 py-2"
    >
      <input id={id} aria-label={label} type="checkbox" checked={checked} onChange={handleChange} />
      {label}
    </label>
  );
}
