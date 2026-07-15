import type { GallerySafetyLabelDto } from "@/types";

type SafetyBadgeProps = {
  label?: GallerySafetyLabelDto | "unknown" | null;
};

export function SafetyBadge({ label = "unknown" }: SafetyBadgeProps) {
  const normalized = label ?? "unknown";
  const classes =
    normalized === "hidden"
      ? "border-rose-500/60 bg-rose-500/12 text-rose-100"
      : normalized === "sensitive"
        ? "border-amber-500/60 bg-amber-500/12 text-amber-100"
        : normalized === "safe"
          ? "border-emerald-500/50 bg-emerald-500/10 text-emerald-100"
          : "border-app-border bg-app-surface text-app-muted";

  return (
    <span
      className={[
        "inline-flex h-7 items-center border px-2 text-[11px] font-bold uppercase tracking-wide",
        classes,
      ].join(" ")}
    >
      {normalized.toUpperCase()}
    </span>
  );
}
