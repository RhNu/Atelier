import type { HTMLAttributes, ReactNode } from "react";

type AppToolbarProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppToolbar({ className = "", children, ...props }: AppToolbarProps) {
  return (
    <div
      className={[
        "flex min-h-12 flex-wrap items-center justify-between gap-3 border-b border-app-border bg-app-panel px-4 py-3",
        className,
      ].join(" ")}
      {...props}
    >
      {children}
    </div>
  );
}
