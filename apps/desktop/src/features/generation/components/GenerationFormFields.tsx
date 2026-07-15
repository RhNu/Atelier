import { useCallback, useId, type ChangeEvent } from "react";

import { AppSelect } from "@/components/ui";

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
