import { useCallback } from "react";

type AppTabItem<TValue extends string> = {
  value: TValue;
  label: string;
  disabled?: boolean;
};

type AppTabsProps<TValue extends string> = {
  value: TValue;
  tabs: ReadonlyArray<AppTabItem<TValue>>;
  onChange: (value: TValue) => void;
  label?: string;
  className?: string;
  fill?: boolean;
};

export function AppTabs<TValue extends string>({
  value,
  tabs,
  onChange,
  label = "Tabs",
  className = "",
  fill = false,
}: AppTabsProps<TValue>) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className={[
        fill ? "flex w-full" : "inline-flex",
        "border border-app-border bg-black/10",
        className,
      ].join(" ")}
    >
      {tabs.map((tab) => (
        <AppTabButton
          key={tab.value}
          selected={tab.value === value}
          tab={tab}
          fill={fill}
          onChange={onChange}
        />
      ))}
    </div>
  );
}

function AppTabButton<TValue extends string>({
  tab,
  selected,
  fill,
  onChange,
}: {
  tab: AppTabItem<TValue>;
  selected: boolean;
  fill: boolean;
  onChange: (value: TValue) => void;
}) {
  const handleClick = useCallback(() => {
    onChange(tab.value);
  }, [onChange, tab.value]);

  return (
    <button
      type="button"
      role="tab"
      aria-selected={selected}
      disabled={tab.disabled}
      className={[
        "h-full min-h-8 border-r border-app-border px-3 text-sm transition-colors last:border-r-0",
        fill ? "flex-1" : "",
        selected
          ? "bg-brand-500/20 text-brand-100"
          : "text-app-muted hover:bg-app-surface hover:text-app-text",
        "disabled:cursor-not-allowed disabled:opacity-50",
      ].join(" ")}
      onClick={handleClick}
    >
      {tab.label}
    </button>
  );
}
