import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useToastStore } from "@/stores/toast-store";

import { AppToastHost } from "./AppToastHost";

beforeEach(() => {
  vi.useFakeTimers();
  useToastStore.setState({ toasts: [] });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("AppToastHost", () => {
  it("dismisses transient notifications automatically", async () => {
    render(<AppToastHost />);
    act(() => {
      useToastStore.getState().push({ level: "success", message: "Saved", durationMs: 100 });
    });
    expect(screen.getByRole("status")).toHaveTextContent("Saved");

    await act(() => vi.advanceTimersByTimeAsync(100));
    expect(screen.queryByText("Saved")).not.toBeInTheDocument();
  });

  it("supports persistent actionable notifications", () => {
    const action = vi.fn<() => void>();
    render(<AppToastHost />);
    act(() => {
      useToastStore.getState().push({
        level: "error",
        message: "Failed",
        durationMs: null,
        action: { label: "Retry", onClick: action },
      });
    });

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(action).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("Failed")).not.toBeInTheDocument();
  });
});
