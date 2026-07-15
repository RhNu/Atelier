import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

type AppIconButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "type"> & {
  icon: LucideIcon;
  label: string;
  selected?: boolean;
  size?: "sm" | "md";
  variant?: "default" | "danger";
};

export function AppIconButton({
  icon: Icon,
  label,
  selected = false,
  size = "md",
  variant = "default",
  className = "",
  ...props
}: AppIconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={[
        "inline-flex items-center justify-center border transition-colors",
        size === "sm" ? "size-8" : "size-10",
        selected
          ? "border-brand-400/70 bg-app-surface text-white"
          : variant === "danger"
            ? "border-transparent text-app-muted hover:border-rose-500/40 hover:bg-rose-500/10 hover:text-rose-100"
            : "border-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      ].join(" ")}
      {...props}
    >
      <Icon aria-hidden="true" className={size === "sm" ? "size-4" : "size-5"} />
    </button>
  );
}
