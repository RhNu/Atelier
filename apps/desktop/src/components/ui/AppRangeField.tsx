import { useCallback, type ChangeEvent, type ReactNode } from "react";

type AppRangeFieldProps = {
  label: string;
  value: number;
  valueText?: string;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  onCommit?: () => void;
  action?: ReactNode;
  className?: string;
};

export function AppRangeField({
  label,
  value,
  valueText = String(value),
  min,
  max,
  step,
  onChange,
  onCommit,
  action,
  className = "",
}: AppRangeFieldProps) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => onChange(Number(event.target.value)),
    [onChange],
  );

  return (
    <label className={["grid gap-1.5 text-xs", className].join(" ")}>
      <span className="flex items-center justify-between gap-3">
        <span className="font-semibold text-app-muted">{label}</span>
        <span className="flex items-center gap-2">
          {action}
          <span className="font-semibold text-app-text tabular-nums">{valueText}</span>
        </span>
      </span>
      <input
        aria-label={label}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        className="w-full accent-brand-500"
        onChange={handleChange}
        onPointerUp={onCommit}
        onKeyUp={onCommit}
        onBlur={onCommit}
      />
    </label>
  );
}
