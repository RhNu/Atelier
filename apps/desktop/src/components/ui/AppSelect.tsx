import type { SelectHTMLAttributes } from "react";

type SelectOption = {
  value: string;
  label: string;
};

type AppSelectProps = SelectHTMLAttributes<HTMLSelectElement> & {
  options: ReadonlyArray<SelectOption>;
};

export function AppSelect({ options, className = "", ...props }: AppSelectProps) {
  return (
    <select
      className={[
        "h-9 w-full border border-app-border bg-app-surface px-3 text-sm text-app-text outline-none",
        "focus:border-brand-400 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      ].join(" ")}
      {...props}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
