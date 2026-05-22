import { useCallback } from "react";

type AppTabItem = {
  value: string;
  label: string;
  disabled?: boolean;
};

type AppTabsProps = {
  value: string;
  tabs: ReadonlyArray<AppTabItem>;
  onChange: (value: string) => void;
  label?: string;
};

export function AppTabs({ value, tabs, onChange, label = "Tabs" }: AppTabsProps) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className="inline-flex border border-app-border bg-black/10"
    >
      {tabs.map((tab) => (
        <AppTabButton
          key={tab.value}
          selected={tab.value === value}
          tab={tab}
          onChange={onChange}
        />
      ))}
    </div>
  );
}

function AppTabButton({
  tab,
  selected,
  onChange,
}: {
  tab: AppTabItem;
  selected: boolean;
  onChange: (value: string) => void;
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
        "h-8 border-r border-app-border px-3 text-sm transition-colors last:border-r-0",
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
