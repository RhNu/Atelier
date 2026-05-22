import type { HTMLAttributes, ReactNode } from "react";

type AppPanelProps = HTMLAttributes<HTMLElement> & {
  as?: "section" | "article" | "aside" | "div" | "nav";
  children: ReactNode;
};

export function AppPanel({
  as: Component = "section",
  className = "",
  children,
  ...props
}: AppPanelProps) {
  return (
    <Component
      className={["border border-app-border bg-app-panel shadow-app-panel", className].join(" ")}
      {...props}
    >
      {children}
    </Component>
  );
}
