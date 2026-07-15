/* eslint-disable jsx-a11y/prefer-tag-over-role, react-perf/jsx-no-new-function-as-prop */
import { forwardRef, useId, type FocusEventHandler, type KeyboardEventHandler } from "react";

import { CompletionList } from "./completion-list";
import { usePromptCompletionController } from "./use-prompt-completion";

type PromptCompletionTextareaProps = {
  id?: string;
  "aria-label": string;
  value: string;
  onChange: (value: string) => void;
  className?: string;
  onBlur?: FocusEventHandler<HTMLTextAreaElement>;
  onKeyDown?: KeyboardEventHandler<HTMLTextAreaElement>;
};

export const PromptCompletionTextarea = forwardRef<
  HTMLTextAreaElement,
  PromptCompletionTextareaProps
>(function PromptCompletionTextarea(
  { id, "aria-label": ariaLabel, value, onChange, className, onBlur, onKeyDown },
  forwardedRef,
) {
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
        ref={(node) => {
          completion.textareaRef.current = node;
          if (typeof forwardedRef === "function") {
            forwardedRef(node);
          } else if (forwardedRef) {
            forwardedRef.current = node;
          }
        }}
        role="combobox"
        aria-label={ariaLabel}
        aria-autocomplete="list"
        aria-expanded={completion.open}
        aria-haspopup="listbox"
        aria-controls={completion.open ? listboxId : undefined}
        aria-activedescendant={activeOptionId}
        value={value}
        onChange={completion.handleChange}
        onKeyDown={(event) => {
          completion.handleKeyDown(event);
          if (!event.defaultPrevented) {
            onKeyDown?.(event);
          }
        }}
        onClick={completion.handleClick}
        onBlur={(event) => {
          completion.handleBlur(event);
          onBlur?.(event);
        }}
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
});
