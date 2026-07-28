import { Component, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { describeError, frontendLogger } from "../app/logger";
import { AppButton, AppPanel } from "../components/ui";

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  error: Error | null;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = {
    error: null,
  };

  handleRetry = () => {
    this.setState({ error: null });
  };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    frontendLogger.error("Atelier frontend render error", {
      error: describeError(error),
      componentStack: errorInfo.componentStack,
    });
  }

  render() {
    if (this.state.error) {
      return <AppErrorFallback error={this.state.error} onRetry={this.handleRetry} />;
    }

    return this.props.children;
  }
}

function AppErrorFallback({ error, onRetry }: { error: Error; onRetry: () => void }) {
  const { t } = useTranslation("shell");
  const { t: translateCommon } = useTranslation("common");
  return (
    <div className="flex h-svh items-center justify-center bg-app-bg p-6">
      <AppPanel className="max-w-xl p-6 shadow-app-panel">
        <p className="text-xs font-semibold text-rose-100 uppercase">{t("frontendError")}</p>
        <h1 className="mt-2 text-lg font-semibold text-white">{t("renderFailed")}</h1>
        <p className="mt-3 text-sm text-app-muted">{error.message}</p>
        <AppButton className="mt-5" variant="secondary" onClick={onRetry}>
          {translateCommon("retry")}
        </AppButton>
      </AppPanel>
    </div>
  );
}
