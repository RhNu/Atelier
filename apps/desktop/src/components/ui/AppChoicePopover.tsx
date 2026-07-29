/* eslint-disable jsx-a11y/prefer-tag-over-role -- The WebView-native listbox is not themeable. */
import { ChevronDown } from "lucide-react";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";

type PopoverPosition = {
  left: number;
  top: number;
  width: number;
  maxHeight: number;
};

type AppChoicePopoverProps = {
  anchorRef: RefObject<HTMLElement | null>;
  id: string;
  label?: string;
  open: boolean;
  children: ReactNode;
  onClose: () => void;
};

const VIEWPORT_PADDING = 8;
const POPOVER_GAP = 4;
const MAX_HEIGHT = 240;
const MIN_HEIGHT = 96;
const MIN_WIDTH = 180;

export function AppChoiceChevron({ open }: { open: boolean }) {
  return (
    <ChevronDown
      aria-hidden="true"
      className={[
        "pointer-events-none absolute top-1/2 right-2 size-4 -translate-y-1/2 text-app-muted transition-transform",
        open ? "rotate-180 text-brand-200" : "",
      ].join(" ")}
    />
  );
}

export function AppChoicePopover({
  anchorRef,
  id,
  label,
  open,
  children,
  onClose,
}: AppChoicePopoverProps) {
  const popoverRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<PopoverPosition>({
    left: 0,
    top: 0,
    width: MIN_WIDTH,
    maxHeight: MAX_HEIGHT,
  });

  const updatePosition = useCallback(() => {
    const anchor = anchorRef.current;
    if (!anchor) return;

    const rect = anchor.getBoundingClientRect();
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
    const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
    const width = Math.min(
      Math.max(rect.width, MIN_WIDTH),
      Math.max(MIN_WIDTH, viewportWidth - VIEWPORT_PADDING * 2),
    );
    const left = Math.min(
      Math.max(VIEWPORT_PADDING, rect.left),
      Math.max(VIEWPORT_PADDING, viewportWidth - width - VIEWPORT_PADDING),
    );
    const below = viewportHeight - rect.bottom - POPOVER_GAP - VIEWPORT_PADDING;
    const above = rect.top - POPOVER_GAP - VIEWPORT_PADDING;
    const placeBelow = below >= MIN_HEIGHT || below >= above;
    const availableHeight = Math.max(MIN_HEIGHT, placeBelow ? below : above);
    const maxHeight = Math.min(MAX_HEIGHT, availableHeight);
    const top = placeBelow
      ? rect.bottom + POPOVER_GAP
      : Math.max(VIEWPORT_PADDING, rect.top - POPOVER_GAP - maxHeight);

    setPosition({ left, top, width, maxHeight });
  }, [anchorRef]);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    window.addEventListener("resize", updatePosition);
    document.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      document.removeEventListener("scroll", updatePosition, true);
    };
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      const target = event.target;
      if (
        target instanceof Node &&
        !anchorRef.current?.contains(target) &&
        !popoverRef.current?.contains(target)
      ) {
        onClose();
      }
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [anchorRef, onClose, open]);

  if (!open) return null;

  return createPortal(
    <div
      ref={popoverRef}
      id={id}
      role="listbox"
      aria-label={label}
      className="fixed z-[70] overflow-y-auto border border-app-border bg-app-panel py-1 text-sm text-app-text shadow-app-panel"
      style={position}
    >
      {children}
    </div>,
    document.body,
  );
}
