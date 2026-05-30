/* eslint-disable jsx-a11y/prefer-tag-over-role */
import { useId } from "react";

import { CompletionList } from "./completion-list";
import { usePromptCompletionController } from "./use-prompt-completion";

type PromptCompletionTextareaProps = {
  id?: string;
  "aria-label": string;
  value: string;
  onChange: (value: string) => void;
  className?: string;
};

export function PromptCompletionTextarea({
  id,
  "aria-label": ariaLabel,
  value,
  onChange,
  className,
}: PromptCompletionTextareaProps) {
  const fallbackListboxId = useId();
  const listboxId = `${id ?? fallbackListboxId}-prompt-completions`;
  const completion = usePromptCompletionController({ value, onChange });
  const activeOptionId =
    completion.open && completion.items[completion.activeIndex]
      ? `${listboxId}-option-${completion.activeIndex}`
      : undefined;

  return (
    <div className="relative">
      <textarea
        id={id}
        ref={completion.textareaRef}
        role="combobox"
        aria-label={ariaLabel}
        aria-autocomplete="list"
        aria-expanded={completion.open}
        aria-haspopup="listbox"
        aria-controls={completion.open ? listboxId : undefined}
        aria-activedescendant={activeOptionId}
        value={value}
        onChange={completion.handleChange}
        onKeyDown={completion.handleKeyDown}
        onClick={completion.handleClick}
        onBlur={completion.handleBlur}
        className={["w-full", className ?? ""].join(" ")}
      />
      {completion.open ? (
        <CompletionList
          id={listboxId}
          items={completion.items}
          activeIndex={completion.activeIndex}
          optionIdPrefix={listboxId}
          manualEmptyPicker={completion.manualEmptyPicker}
          onAccept={completion.acceptItem}
        />
      ) : null}
    </div>
  );
}
