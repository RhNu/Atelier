import { Bot } from "lucide-react";
import { useTranslation } from "react-i18next";

export function AgentNavButton({
  open,
  running,
  onClick,
}: {
  open: boolean;
  running: boolean;
  onClick?: () => void;
}) {
  const { t } = useTranslation("shell");
  return (
    <button
      type="button"
      aria-label={t("openAgent")}
      aria-pressed={open}
      title={`${t("openAgent")} (Ctrl+Shift+A)`}
      className={[
        "relative flex h-11 w-full items-center justify-center border-y border-transparent transition-colors",
        open
          ? "border-app-border bg-app-bg text-brand-100"
          : "text-app-muted hover:bg-app-surface hover:text-app-text",
      ].join(" ")}
      onClick={onClick}
    >
      <Bot aria-hidden="true" className="size-5" />
      {running ? (
        <span
          aria-label={t("agentRunning")}
          className="absolute top-2 right-2 size-1.5 animate-pulse bg-brand-300"
        />
      ) : null}
    </button>
  );
}
