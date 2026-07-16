import { ChevronDown } from "lucide-react";
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
    <span className="relative block w-full">
      <select
        className={[
          "h-9 w-full appearance-none border border-app-border bg-app-surface px-3 pr-8 text-sm text-app-text outline-none",
          "[&>option]:bg-app-surface [&>option]:text-app-text",
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
      <ChevronDown
        aria-hidden="true"
        className="pointer-events-none absolute top-1/2 right-2 size-4 -translate-y-1/2 text-app-muted"
      />
    </span>
  );
}
