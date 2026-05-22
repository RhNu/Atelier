import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

type AppIconButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "type"> & {
  icon: LucideIcon;
  label: string;
  selected?: boolean;
};

export function AppIconButton({
  icon: Icon,
  label,
  selected = false,
  className = "",
  ...props
}: AppIconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={[
        "inline-flex size-10 items-center justify-center border transition-colors",
        selected
          ? "border-brand-400/70 bg-app-surface text-white"
          : "border-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      ].join(" ")}
      {...props}
    >
      <Icon aria-hidden="true" className="size-5" />
    </button>
  );
}
