/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-array-as-prop */
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { CharacterPositionGrid } from "../features/generation/components/CharacterPositionGrid";
import { GenerationWorkbenchLayout } from "../features/generation/components/GenerationWorkbenchLayout";
import type { GenerationCharacterDraft } from "../features/generation/model/generation-draft";

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
    const resizer = screen.getByRole("separator", { name: "Resize generation settings" });
    expect(sidebar).toHaveStyle({ width: "420px" });

    fireEvent.keyDown(resizer, { key: "ArrowRight" });
    expect(sidebar).toHaveStyle({ width: "436px" });
    expect(window.localStorage.getItem(SIDEBAR_STORAGE_KEY)).toBe("436");

    fireEvent.keyDown(resizer, { key: "End" });
    expect(sidebar).toHaveStyle({ width: "520px" });
    fireEvent.keyDown(resizer, { key: "Home" });
    expect(sidebar).toHaveStyle({ width: "360px" });
    expect(window.localStorage.getItem(SIDEBAR_STORAGE_KEY)).toBe("360");
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
    expect(screen.getByTestId("generation-settings-sidebar")).toHaveStyle({ width: "520px" });
  });
});

describe("CharacterPositionGrid", () => {
  it("selects characters, snaps cells, and supports keyboard movement", async () => {
    const user = userEvent.setup();
    const onSelectCharacter = vi.fn<(index: number) => void>();
    const onChangePosition = vi.fn<(index: number, position: { x: number; y: number }) => void>();
    const characters: GenerationCharacterDraft[] = [
      character("one", 0.5, 0.5),
      character("two", 0.75, 0.75),
    ];
    render(
      <CharacterPositionGrid
        characters={characters}
        activeIndex={0}
        onSelectCharacter={onSelectCharacter}
        onChangePosition={onChangePosition}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Select character 2" }));
    expect(onSelectCharacter).toHaveBeenCalledWith(1);

    await user.click(screen.getByRole("gridcell", { name: "Position 25%, 75%" }));
    expect(onChangePosition).toHaveBeenCalledWith(0, { x: 0.25, y: 0.75 });

    const grid = screen.getByRole("grid", { name: "Character position grid" });
    fireEvent.keyDown(grid, { key: "ArrowLeft" });
    expect(onChangePosition).toHaveBeenLastCalledWith(0, { x: 0.25, y: 0.5 });
    fireEvent.keyDown(grid, { key: "Home" });
    expect(onChangePosition).toHaveBeenLastCalledWith(0, { x: 0.5, y: 0.5 });
  });
});

function character(id: string, x: number, y: number): GenerationCharacterDraft {
  return {
    id,
    presetId: null,
    prompt: "",
    negativePrompt: "",
    enabled: true,
    position: { x, y },
  };
}
