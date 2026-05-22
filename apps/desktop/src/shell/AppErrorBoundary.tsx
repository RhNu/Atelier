import { Component, type ErrorInfo, type ReactNode } from "react";

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
    console.error("Atelier frontend error", error, errorInfo);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-svh items-center justify-center bg-app-bg p-6">
          <AppPanel className="max-w-xl p-6">
            <p className="text-xs font-semibold text-rose-100 uppercase">Frontend error</p>
            <h1 className="mt-2 text-lg font-semibold text-white">
              Atelier could not render this view
            </h1>
            <p className="mt-3 text-sm text-app-muted">{this.state.error.message}</p>
            <AppButton className="mt-5" variant="secondary" onClick={this.handleRetry}>
              Retry
            </AppButton>
          </AppPanel>
        </div>
      );
    }

    return this.props.children;
  }
}
