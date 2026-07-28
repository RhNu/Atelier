import { CircleHelp } from "lucide-react";
import {
  useCallback,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent,
  type MouseEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

const TOOLTIP_GUTTER = 8;
const TOOLTIP_WIDTH = 224;
const TOOLTIP_HEIGHT = 72;

export type AppHelpMarkerAlign = "start" | "center" | "end";
export type AppHelpMarkerSide = "top" | "bottom";

export function AppHelpMarker({
  content,
  label = "Help",
  hoverOnly = false,
  align = "center",
  side = "bottom",
  offset = 4,
}: {
  content: ReactNode;
  label?: string;
  hoverOnly?: boolean;
  /** Horizontal alignment relative to the marker before viewport collision handling. */
  align?: AppHelpMarkerAlign;
  /** Preferred side of the marker. The tooltip flips when that side has insufficient space. */
  side?: AppHelpMarkerSide;
  /** Distance in pixels between the marker and the tooltip. */
  offset?: number;
}) {
  const markerRef = useRef<HTMLSpanElement>(null);
  const tooltipRef = useRef<HTMLSpanElement>(null);
  const tooltipId = useId();
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [position, setPosition] = useState<CSSProperties>({
    left: TOOLTIP_GUTTER,
    top: TOOLTIP_GUTTER,
  });
  const open = hovered || focused;

  const updatePosition = useCallback(() => {
    const marker = markerRef.current;
    const tooltip = tooltipRef.current;
    if (!marker || !tooltip) {
      return;
    }

    const markerRect = marker.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    const tooltipWidth = tooltipRect.width || TOOLTIP_WIDTH;
    const tooltipHeight = tooltipRect.height || TOOLTIP_HEIGHT;
    const horizontalSpace = window.innerWidth - TOOLTIP_GUTTER * 2;
    const verticalSpace = window.innerHeight - TOOLTIP_GUTTER * 2;
    const availableWidth = Math.max(0, Math.min(tooltipWidth, horizontalSpace));
    const availableHeight = Math.max(0, Math.min(tooltipHeight, verticalSpace));

    const alignedLeft =
      align === "start"
        ? markerRect.left
        : align === "end"
          ? markerRect.right - availableWidth
          : markerRect.left + (markerRect.width - availableWidth) / 2;
    const left = clamp(
      alignedLeft,
      TOOLTIP_GUTTER,
      Math.max(TOOLTIP_GUTTER, window.innerWidth - availableWidth - TOOLTIP_GUTTER),
    );
    const preferredTop =
      side === "top" ? markerRect.top - availableHeight - offset : markerRect.bottom + offset;
    const alternateTop =
      side === "top" ? markerRect.bottom + offset : markerRect.top - availableHeight - offset;
    const preferredFits =
      preferredTop >= TOOLTIP_GUTTER &&
      preferredTop + availableHeight <= window.innerHeight - TOOLTIP_GUTTER;
    const alternateFits =
      alternateTop >= TOOLTIP_GUTTER &&
      alternateTop + availableHeight <= window.innerHeight - TOOLTIP_GUTTER;
    const top = preferredFits || !alternateFits ? preferredTop : alternateTop;

    setPosition({
      left,
      top: clamp(
        top,
        TOOLTIP_GUTTER,
        Math.max(TOOLTIP_GUTTER, window.innerHeight - availableHeight - TOOLTIP_GUTTER),
      ),
      maxWidth: `calc(100vw - ${TOOLTIP_GUTTER * 2}px)`,
    });
  }, [align, offset, side]);

  useLayoutEffect(() => {
    if (!open) {
      return;
    }

    updatePosition();
    const handleViewportChange = () => updatePosition();
    window.addEventListener("resize", handleViewportChange);
    window.addEventListener("scroll", handleViewportChange, true);
    return () => {
      window.removeEventListener("resize", handleViewportChange);
      window.removeEventListener("scroll", handleViewportChange, true);
    };
  }, [content, open, updatePosition]);

  const handleMouseEnter = useCallback(() => setHovered(true), []);
  const handleMouseLeave = useCallback((event: MouseEvent<HTMLSpanElement>) => {
    if (
      !(event.relatedTarget instanceof Node) ||
      !event.currentTarget.contains(event.relatedTarget)
    ) {
      setHovered(false);
    }
  }, []);
  const handleFocus = useCallback(() => setFocused(true), []);
  const handleBlur = useCallback((event: FocusEvent<HTMLSpanElement>) => {
    if (
      !(event.relatedTarget instanceof Node) ||
      !event.currentTarget.contains(event.relatedTarget)
    ) {
      setFocused(false);
    }
  }, []);

  const tooltip = (
    <span
      ref={tooltipRef}
      id={tooltipId}
      role="tooltip"
      style={position}
      className={[
        "pointer-events-none fixed z-[60] hidden w-56 border border-app-border bg-app-panel p-2 text-left text-xs leading-5 font-normal text-app-text normal-case shadow-app-panel",
        open ? "!block" : "",
      ].join(" ")}
    >
      {content}
    </span>
  );

  return (
    <>
      <span
        ref={markerRef}
        className="relative inline-flex shrink-0"
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={handleFocus}
        onBlur={handleBlur}
      >
        {hoverOnly ? (
          <span
            aria-describedby={tooltipId}
            aria-label={label}
            className="grid size-5 place-items-center text-app-muted"
          >
            <CircleHelp aria-hidden="true" className="size-3.5" />
          </span>
        ) : (
          <button
            type="button"
            aria-describedby={tooltipId}
            aria-label={label}
            className="grid size-5 place-items-center text-app-muted outline-none hover:text-app-text focus-visible:text-app-text"
          >
            <CircleHelp aria-hidden="true" className="size-3.5" />
          </button>
        )}
      </span>
      {typeof document === "undefined" ? null : createPortal(tooltip, document.body)}
    </>
  );
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}
