/* eslint-disable react-perf/jsx-no-new-function-as-prop, jsx-a11y/prefer-tag-over-role */
import { Check } from "lucide-react";
import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type KeyboardEvent,
} from "react";

import { AppChoiceChevron, AppChoicePopover } from "./AppChoicePopover";

export type SelectOption = {
  value: string;
  label: string;
};

export type SelectOptionGroup = {
  type: "group";
  label: string;
  options: ReadonlyArray<SelectOption>;
};

type AppSelectProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children" | "onChange" | "value"
> & {
  value: string;
  options: ReadonlyArray<SelectOption | SelectOptionGroup>;
  containerClassName?: string;
  onValueChange?: (value: string) => void;
};

type FlatOption = SelectOption & {
  groupLabel?: string;
};

export function AppSelect({
  value,
  options,
  className = "",
  containerClassName = "",
  disabled,
  name,
  onClick,
  onKeyDown,
  onValueChange,
  ...props
}: AppSelectProps) {
  const generatedId = useId();
  const listboxId = `${generatedId}-listbox`;
  const anchorRef = useRef<HTMLSpanElement>(null);
  const typeaheadRef = useRef("");
  const typeaheadTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const flatOptions = useMemo(() => flattenOptions(options), [options]);
  const selectedIndex = flatOptions.findIndex((option) => option.value === value);
  const selectedOption = flatOptions[selectedIndex];
  const safeActiveIndex = Math.min(activeIndex, Math.max(0, flatOptions.length - 1));

  const close = useCallback(() => setOpen(false), []);
  const openList = useCallback(() => {
    if (disabled || flatOptions.length === 0) return;
    setActiveIndex(Math.max(0, selectedIndex));
    setOpen(true);
  }, [disabled, flatOptions.length, selectedIndex]);

  useEffect(
    () => () => {
      if (typeaheadTimerRef.current) clearTimeout(typeaheadTimerRef.current);
    },
    [],
  );

  function selectOption(option: FlatOption) {
    if (option.value !== value) onValueChange?.(option.value);
    setOpen(false);
    anchorRef.current?.querySelector("button")?.focus();
  }

  function moveActive(offset: number) {
    setActiveIndex((current) => (current + offset + flatOptions.length) % flatOptions.length);
  }

  function handleTypeahead(key: string) {
    if (typeaheadTimerRef.current) clearTimeout(typeaheadTimerRef.current);
    typeaheadRef.current += key.toLocaleLowerCase();
    const matchIndex = flatOptions.findIndex((option) =>
      option.label.toLocaleLowerCase().startsWith(typeaheadRef.current),
    );
    if (matchIndex >= 0) {
      setActiveIndex(matchIndex);
      setOpen(true);
    }
    typeaheadTimerRef.current = setTimeout(() => {
      typeaheadRef.current = "";
    }, 500);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    onKeyDown?.(event);
    if (event.defaultPrevented || flatOptions.length === 0) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        if (open) moveActive(1);
        else openList();
        break;
      case "ArrowUp":
        event.preventDefault();
        if (open) moveActive(-1);
        else openList();
        break;
      case "Home":
        event.preventDefault();
        setActiveIndex(0);
        setOpen(true);
        break;
      case "End":
        event.preventDefault();
        setActiveIndex(flatOptions.length - 1);
        setOpen(true);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (open) selectOption(flatOptions[safeActiveIndex]);
        else openList();
        break;
      case "Escape":
        if (open) {
          event.preventDefault();
          event.stopPropagation();
          setOpen(false);
        }
        break;
      default:
        if (event.key.length === 1 && !event.altKey && !event.ctrlKey && !event.metaKey) {
          handleTypeahead(event.key);
        }
    }
  }

  return (
    <span ref={anchorRef} className={["relative block w-full", containerClassName].join(" ")}>
      <button
        {...props}
        type="button"
        role="combobox"
        aria-controls={open ? listboxId : undefined}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-activedescendant={open ? `${generatedId}-option-${safeActiveIndex}` : undefined}
        value={value}
        disabled={disabled}
        className={[
          "flex h-9 w-full items-center border border-app-border bg-app-surface px-3 pr-8 text-left text-sm text-app-text outline-none",
          "focus:border-brand-400 disabled:cursor-not-allowed disabled:opacity-50",
          className,
        ].join(" ")}
        onClick={(event) => {
          onClick?.(event);
          if (!event.defaultPrevented) {
            if (open) setOpen(false);
            else openList();
          }
        }}
        onKeyDown={handleKeyDown}
      >
        <span className="min-w-0 flex-1 truncate">{selectedOption?.label ?? value}</span>
      </button>
      {name ? <input type="hidden" name={name} value={value} /> : null}
      <AppChoiceChevron open={open} />
      <AppChoicePopover
        anchorRef={anchorRef}
        id={listboxId}
        label={props["aria-label"]}
        open={open}
        onClose={close}
      >
        <AppSelectOptions
          options={options}
          flatOptions={flatOptions}
          generatedId={generatedId}
          value={value}
          activeIndex={safeActiveIndex}
          onActiveIndexChange={setActiveIndex}
          onSelect={selectOption}
        />
      </AppChoicePopover>
    </span>
  );
}

function AppSelectOptions({
  options,
  flatOptions,
  generatedId,
  value,
  activeIndex,
  onActiveIndexChange,
  onSelect,
}: {
  options: ReadonlyArray<SelectOption | SelectOptionGroup>;
  flatOptions: ReadonlyArray<FlatOption>;
  generatedId: string;
  value: string;
  activeIndex: number;
  onActiveIndexChange: (index: number) => void;
  onSelect: (option: FlatOption) => void;
}) {
  function renderOption(option: SelectOption, groupLabel?: string) {
    const index = flatOptions.findIndex(
      (item) => item.value === option.value && item.groupLabel === groupLabel,
    );
    return (
      <div
        key={option.value}
        id={`${generatedId}-option-${index}`}
        role="option"
        aria-selected={option.value === value}
        className={[
          "flex min-h-8 cursor-default items-center gap-2 px-3 py-1.5",
          index === activeIndex ? "bg-brand-500/20 text-brand-100" : "hover:bg-white/5",
        ].join(" ")}
        onPointerMove={() => onActiveIndexChange(index)}
        onPointerDown={(event) => {
          event.preventDefault();
          onSelect({ ...option, groupLabel });
        }}
      >
        <span className="min-w-0 flex-1 truncate">{option.label}</span>
        {option.value === value ? <Check aria-hidden="true" className="size-4 shrink-0" /> : null}
      </div>
    );
  }

  return options.map((option) =>
    "type" in option ? (
      <div key={option.label} role="group" aria-label={option.label}>
        <div className="px-3 pt-2 pb-1 text-[10px] font-bold tracking-wide text-app-muted uppercase">
          {option.label}
        </div>
        {option.options.map((groupOption) => renderOption(groupOption, option.label))}
      </div>
    ) : (
      renderOption(option)
    ),
  );
}

function flattenOptions(options: ReadonlyArray<SelectOption | SelectOptionGroup>): FlatOption[] {
  return options.flatMap((option) =>
    "type" in option
      ? option.options.map((groupOption) => ({ ...groupOption, groupLabel: option.label }))
      : [option],
  );
}
