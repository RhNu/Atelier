import type { HTMLAttributes, ReactNode } from "react";

type AppPanelProps = HTMLAttributes<HTMLElement> & {
  as?: "section" | "article" | "aside" | "div" | "nav";
  children: ReactNode;
  variant?: "panel" | "section";
};

export function AppPanel({
  as: Component = "section",
  className = "",
  children,
  variant = "panel",
  ...props
}: AppPanelProps) {
  return (
    <Component
      className={[
        variant === "section" ? "bg-app-panel" : "border border-app-border bg-app-panel",
        className,
      ].join(" ")}
      {...props}
    >
      {children}
    </Component>
  );
}
