/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { render, screen } from "@testing-library/react";

import {
  DirectorInputPanel,
  DirectorPreviewPanel,
  DirectorRunPanel,
} from "../features/director/components/DirectorPanels";
import type { DirectorToolResultDto } from "../types";

const result: DirectorToolResultDto = {
  item_id: "director-item",
  artifact_id: "director-artifact",
  resource: { id: "director-output", variant_id: null },
  item: {
    item_id: "director-item",
    artifact_id: "director-artifact",
    artifact_kind: "director_result",
    source_kind: "director",
    primary_resource: { id: "director-output", variant_id: null },
    assets: [],
    indexed_at_ms: 1,
    seed: null,
    request_seed: null,
    prompt: null,
    negative_prompt: null,
    embedded_metadata_status: null,
    embedded_metadata_error: null,
    embedded_metadata_warnings: [],
    sample_index: null,
    model_name: null,
    safety: null,
    manual_safety_override: null,
  },
};

describe("Director panels", () => {
  it("shows only an icon when no input is selected", () => {
    render(
      <DirectorInputPanel
        input={null}
        imageSrc={null}
        loadingImage={false}
        imageError={null}
        pickPending={false}
        onPick={() => undefined}
        onPaste={() => undefined}
        onClear={() => undefined}
      />,
    );

    expect(screen.getByRole("img", { name: "No director input" })).toBeInTheDocument();
    expect(screen.queryByText("No director input")).not.toBeInTheDocument();
    expect(screen.queryByText("Import or paste an image.")).not.toBeInTheDocument();
  });

  it("shows a single Output comparison panel without a duplicate Original", () => {
    render(<DirectorPreviewPanel resultSrc={null} resultPending={false} resultError={null} />);

    expect(screen.getByRole("heading", { name: "Output" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Original" })).not.toBeInTheDocument();
    expect(screen.getByText("No image").closest("section")).toHaveClass("h-full");
  });

  it("shows compact localized tools and only the refreshable Anlas balance", () => {
    render(
      <DirectorRunPanel
        tool="lineart"
        anlas={1200}
        showsPrompt={false}
        promptRequired={false}
        prompt=""
        defry={0}
        canRun
        runPending={false}
        result={result}
        safetyOverride=""
        readinessPending={false}
        readinessError={null}
        savePending={false}
        safetyPending={false}
        onToolChange={() => undefined}
        onPromptChange={() => undefined}
        onDefryChange={() => undefined}
        onRefresh={() => undefined}
        onRun={() => undefined}
        onSave={() => undefined}
        onSafetyChange={() => undefined}
        onApplySafety={() => undefined}
      />,
    );

    expect(screen.queryByRole("heading", { name: "Controls" })).not.toBeInTheDocument();
    expect(screen.queryByText("Opus")).not.toBeInTheDocument();
    expect(screen.getByText("1200 Anlas")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh Anlas" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Lineart" })).toBeChecked();
    expect(screen.getByText("Extract clean line art")).toBeInTheDocument();
    expect(screen.getByText("Advanced settings").closest("details")).not.toHaveAttribute("open");
  });

  it("uses the backend defry bounds for prompt-capable tools", () => {
    render(
      <DirectorRunPanel
        tool="colorize"
        anlas={1200}
        showsPrompt
        promptRequired={false}
        prompt=""
        defry={2}
        canRun
        runPending={false}
        result={null}
        safetyOverride=""
        readinessPending={false}
        readinessError={null}
        savePending={false}
        safetyPending={false}
        onToolChange={() => undefined}
        onPromptChange={() => undefined}
        onDefryChange={() => undefined}
        onRefresh={() => undefined}
        onRun={() => undefined}
        onSave={() => undefined}
        onSafetyChange={() => undefined}
        onApplySafety={() => undefined}
      />,
    );

    const slider = screen.getByRole("slider", { name: "Defry" });
    expect(slider).toHaveAttribute("min", "0");
    expect(slider).toHaveAttribute("max", "5");
    expect(slider).toHaveAttribute("step", "1");
  });
});
