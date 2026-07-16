import { useCallback, type ChangeEvent } from "react";

export const MAX_SEED = Number.MAX_SAFE_INTEGER;

export function SeedInput({
  label,
  value,
  randomPlaceholder,
  onChange,
  onBlur,
}: {
  label: string;
  value: number;
  randomPlaceholder: string;
  onChange: (value: number) => void;
  onBlur?: () => void;
}) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const raw = event.target.value;
      if (raw === "") {
        onChange(0);
        return;
      }
      if (!/^\d+$/u.test(raw)) return;
      const next = Number(raw);
      if (Number.isSafeInteger(next) && next >= 1 && next <= MAX_SEED) onChange(next);
    },
    [onChange],
  );

  return (
    <label className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <input
        aria-label={label}
        type="number"
        inputMode="numeric"
        min={1}
        max={MAX_SEED}
        step={1}
        placeholder={randomPlaceholder}
        value={value === 0 ? "" : String(value)}
        className="h-9 border border-app-border bg-black/20 px-3 text-sm font-normal text-app-text normal-case outline-none focus:border-brand-400"
        onChange={handleChange}
        onBlur={onBlur}
      />
    </label>
  );
}
