/* eslint-disable react-perf/jsx-no-new-function-as-prop, jsx-a11y/prefer-tag-over-role, jsx-a11y/no-redundant-roles */
import { Check } from "lucide-react";
import {
  useCallback,
  useId,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type InputHTMLAttributes,
  type KeyboardEvent,
} from "react";

import { AppChoiceChevron, AppChoicePopover } from "./AppChoicePopover";

type AppComboboxProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "children" | "list" | "onChange" | "role" | "value"
> & {
  value: string;
  suggestions: ReadonlyArray<string>;
  containerClassName?: string;
  onValueChange: (value: string) => void;
};

export function AppCombobox({
  value,
  suggestions,
  className = "",
  containerClassName = "",
  disabled,
  onBlur,
  onFocus,
  onKeyDown,
  onValueChange,
  ...props
}: AppComboboxProps) {
  const generatedId = useId();
  const listboxId = `${generatedId}-listbox`;
  const anchorRef = useRef<HTMLSpanElement>(null);
  const composingRef = useRef(false);
  const [open, setOpen] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const filteredSuggestions = useMemo(
    () => (showAll ? [...suggestions] : filterSuggestions(suggestions, value)),
    [showAll, suggestions, value],
  );
  const expanded = open && filteredSuggestions.length > 0;
  const safeActiveIndex = Math.min(activeIndex, Math.max(0, filteredSuggestions.length - 1));
  const activeOptionId = expanded ? `${generatedId}-option-${safeActiveIndex}` : undefined;

  const close = useCallback(() => setOpen(false), []);
  const openList = useCallback(() => {
    if (disabled || suggestions.length === 0) return;
    const selectedIndex = suggestions.findIndex(
      (suggestion) => suggestion.toLocaleLowerCase() === value.toLocaleLowerCase(),
    );
    setActiveIndex(Math.max(0, selectedIndex));
    setShowAll(true);
    setOpen(true);
  }, [disabled, suggestions, value]);

  function selectSuggestion(suggestion: string) {
    onValueChange(suggestion);
    setOpen(false);
  }

  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    onValueChange(event.target.value);
    setActiveIndex(0);
    setShowAll(false);
    setOpen(suggestions.length > 0);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    onKeyDown?.(event);
    if (event.defaultPrevented || composingRef.current) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        if (!expanded) {
          openList();
        } else {
          setActiveIndex((current) => (current + 1) % filteredSuggestions.length);
        }
        break;
      case "ArrowUp":
        event.preventDefault();
        if (!expanded) {
          openList();
        } else {
          setActiveIndex(
            (current) => (current - 1 + filteredSuggestions.length) % filteredSuggestions.length,
          );
        }
        break;
      case "Home":
        if (expanded) {
          event.preventDefault();
          setActiveIndex(0);
        }
        break;
      case "End":
        if (expanded) {
          event.preventDefault();
          setActiveIndex(filteredSuggestions.length - 1);
        }
        break;
      case "Enter":
        if (expanded) {
          event.preventDefault();
          selectSuggestion(filteredSuggestions[safeActiveIndex]);
        }
        break;
      case "Escape":
        if (expanded) {
          event.preventDefault();
          event.stopPropagation();
          setOpen(false);
        }
        break;
    }
  }

  return (
    <span ref={anchorRef} className={["relative block w-full", containerClassName].join(" ")}>
      <input
        {...props}
        role="combobox"
        aria-autocomplete="list"
        aria-controls={expanded ? listboxId : undefined}
        aria-expanded={expanded}
        aria-activedescendant={activeOptionId}
        autoComplete="off"
        value={value}
        disabled={disabled}
        className={[
          "h-9 w-full border border-app-border bg-black/20 px-3 pr-8 text-sm font-normal text-app-text outline-none",
          "focus:border-brand-400 disabled:cursor-not-allowed disabled:opacity-50",
          className,
        ].join(" ")}
        onChange={handleChange}
        onFocus={(event) => {
          onFocus?.(event);
          if (!event.defaultPrevented) openList();
        }}
        onClick={openList}
        onBlur={(event) => {
          onBlur?.(event);
          setOpen(false);
        }}
        onKeyDown={handleKeyDown}
        onCompositionStart={() => {
          composingRef.current = true;
        }}
        onCompositionEnd={() => {
          composingRef.current = false;
        }}
      />
      <AppChoiceChevron open={expanded} />
      <AppChoicePopover
        anchorRef={anchorRef}
        id={listboxId}
        label={props["aria-label"]}
        open={expanded}
        onClose={close}
      >
        <ComboboxOptions
          suggestions={filteredSuggestions}
          value={value}
          generatedId={generatedId}
          activeIndex={safeActiveIndex}
          onActiveIndexChange={setActiveIndex}
          onSelect={selectSuggestion}
        />
      </AppChoicePopover>
    </span>
  );
}

function ComboboxOptions({
  suggestions,
  value,
  generatedId,
  activeIndex,
  onActiveIndexChange,
  onSelect,
}: {
  suggestions: ReadonlyArray<string>;
  value: string;
  generatedId: string;
  activeIndex: number;
  onActiveIndexChange: (index: number) => void;
  onSelect: (suggestion: string) => void;
}) {
  return suggestions.map((suggestion, index) => {
    const selected = suggestion.toLocaleLowerCase() === value.toLocaleLowerCase();
    const active = index === activeIndex;
    return (
      <div
        key={suggestion}
        id={`${generatedId}-option-${index}`}
        role="option"
        aria-selected={selected}
        className={[
          "flex min-h-8 cursor-default items-center gap-2 px-3 py-1.5",
          active ? "bg-brand-500/20 text-brand-100" : "hover:bg-white/5",
        ].join(" ")}
        onPointerMove={() => onActiveIndexChange(index)}
        onPointerDown={(event) => {
          event.preventDefault();
          onSelect(suggestion);
        }}
      >
        <span className="min-w-0 flex-1 truncate">{suggestion}</span>
        {selected ? <Check aria-hidden="true" className="size-4 shrink-0" /> : null}
      </div>
    );
  });
}

function filterSuggestions(suggestions: ReadonlyArray<string>, query: string): string[] {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  if (!normalizedQuery) return [...suggestions];

  return suggestions
    .filter((suggestion) => suggestion.toLocaleLowerCase().includes(normalizedQuery))
    .toSorted((left, right) => {
      const leftStarts = left.toLocaleLowerCase().startsWith(normalizedQuery);
      const rightStarts = right.toLocaleLowerCase().startsWith(normalizedQuery);
      return Number(rightStarts) - Number(leftStarts);
    });
}
