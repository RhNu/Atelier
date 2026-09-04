/* eslint-disable react-perf/jsx-no-jsx-as-prop */
import { fireEvent, render, screen } from "@testing-library/react";

import { GenerationWorkbenchLayout } from "../features/generation/components/GenerationWorkbenchLayout";
import { i18n } from "../i18n";

const SIDEBAR_STORAGE_KEY = "atelier.ui.generate.sidebar-width.v1";

beforeEach(() => {
  window.localStorage.clear();
});

describe("GenerationWorkbenchLayout", () => {
  it("supports keyboard resizing and remembers the width", async () => {
    render(
      <GenerationWorkbenchLayout
        sidebar={<div>Sidebar</div>}
        preview={<div>Preview</div>}
        history={<div>History</div>}
      />,
    );

    const sidebar = screen.getByTestId("generation-settings-sidebar");
    const resizer = screen.getByRole("separator", { name: i18n.t("generation:resizeSettings") });
    expect(sidebar).toHaveStyle({ width: "360px" });

    fireEvent.keyDown(resizer, { key: "ArrowRight" });
    expect(sidebar).toHaveStyle({ width: "376px" });
    expect(window.localStorage.getItem(SIDEBAR_STORAGE_KEY)).toBe("376");

    fireEvent.keyDown(resizer, { key: "End" });
    expect(sidebar).toHaveStyle({ width: "480px" });
    fireEvent.keyDown(resizer, { key: "Home" });
    expect(sidebar).toHaveStyle({ width: "320px" });
    expect(window.localStorage.getItem(SIDEBAR_STORAGE_KEY)).toBe("320");
  });

  it("hydrates a stored width and clamps invalid extremes", () => {
    window.localStorage.setItem(SIDEBAR_STORAGE_KEY, "999");
    render(
      <GenerationWorkbenchLayout
        sidebar={<div>Sidebar</div>}
        preview={<div>Preview</div>}
        history={<div>History</div>}
      />,
    );
    expect(screen.getByTestId("generation-settings-sidebar")).toHaveStyle({ width: "480px" });
  });
});
