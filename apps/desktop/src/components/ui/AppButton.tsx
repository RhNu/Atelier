import type { ButtonHTMLAttributes, ReactNode } from "react";

type AppButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  children: ReactNode;
};

const variantClasses = {
  primary:
    "border-brand-500 bg-brand-500 text-white hover:bg-brand-400 disabled:hover:bg-brand-500",
  secondary:
    "border-app-border bg-app-surface text-app-text hover:border-brand-400/60 hover:text-white",
  ghost:
    "border-transparent bg-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
  danger: "border-rose-500/60 bg-rose-500/12 text-rose-100 hover:bg-rose-500/20",
} as const;

export function AppButton({
  variant = "primary",
  className = "",
  children,
  type = "button",
  ...props
}: AppButtonProps) {
  return (
    <button
      type={type}
      className={[
        "inline-flex h-9 items-center justify-center gap-2 border px-3 text-sm font-semibold transition-colors",
        "disabled:cursor-not-allowed disabled:opacity-50",
        variantClasses[variant],
        className,
      ].join(" ")}
      {...props}
    >
      {children}
    </button>
  );
}
