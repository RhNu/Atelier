/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { render, screen } from "@testing-library/react";

import {
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
    sample_index: null,
    model_name: null,
    safety: null,
    manual_safety_override: null,
  },
};

describe("Director panels", () => {
  it("shows a single Output comparison panel without a duplicate Original", () => {
    render(<DirectorPreviewPanel resultSrc={null} resultPending={false} resultError={null} />);

    expect(screen.getByRole("heading", { name: "Output" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Original" })).not.toBeInTheDocument();
  });

  it("moves account status into Controls and keeps safety settings collapsed", () => {
    render(
      <DirectorRunPanel
        tool="lineart"
        toolDescription="Extract line art"
        tier="Opus"
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
        onRun={() => undefined}
        onSave={() => undefined}
        onSafetyChange={() => undefined}
        onApplySafety={() => undefined}
      />,
    );

    expect(screen.getByRole("heading", { name: "Controls" })).toBeInTheDocument();
    expect(screen.getByText("Opus")).toBeInTheDocument();
    expect(screen.getByText("1200 Anlas")).toBeInTheDocument();
    expect(screen.getByText("Advanced settings").closest("details")).not.toHaveAttribute("open");
  });
});
