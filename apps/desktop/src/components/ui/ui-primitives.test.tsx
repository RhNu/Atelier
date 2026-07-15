import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Sparkles } from "lucide-react";

import { AppButton } from "./AppButton";
import { AppHelpMarker } from "./AppHelpMarker";
import { AppIconButton } from "./AppIconButton";
import { AppRangeField } from "./AppRangeField";
import { AppTabs } from "./AppTabs";
import { EmptyState } from "./EmptyState";
import { SafetyBadge } from "./SafetyBadge";

const tabItems = [
  { value: "generate", label: "Generate" },
  { value: "gallery", label: "Gallery" },
] as const;

describe("UI primitives", () => {
  it("renders accessible command and icon buttons", async () => {
    const onClick = vi.fn<() => void>();

    render(
      <>
        <AppButton onClick={onClick}>Generate</AppButton>
        <AppIconButton label="Preview prompt" icon={Sparkles} onClick={onClick} />
      </>,
    );

    await userEvent.click(screen.getByRole("button", { name: "Generate" }));
    await userEvent.click(screen.getByRole("button", { name: "Preview prompt" }));

    expect(onClick).toHaveBeenCalledTimes(2);
  });

  it("renders tabs and selected safety state", () => {
    const onChange = vi.fn<(value: string) => void>();

    render(
      <>
        <AppTabs value="gallery" tabs={tabItems} onChange={onChange} />
        <SafetyBadge label="sensitive" />
        <EmptyState title="No artifacts" description="Generate images to populate this view." />
      </>,
    );

    expect(screen.getByRole("tab", { name: "Gallery", selected: true })).toBeInTheDocument();
    expect(screen.getByText("SENSITIVE")).toBeInTheDocument();
    expect(screen.getByText("No artifacts")).toBeInTheDocument();
  });

  it("renders compact help and controlled range primitives", () => {
    const onChange = vi.fn<(value: number) => void>();
    const onCommit = vi.fn<() => void>();

    render(
      <>
        <AppHelpMarker label="Strength help" content="Controls NovelAI guidance strength." />
        <AppRangeField
          label="Strength"
          value={0.6}
          valueText="0.60"
          min={0}
          max={1}
          step={0.01}
          onChange={onChange}
          onCommit={onCommit}
        />
        <AppIconButton label="Remove" icon={Sparkles} size="sm" variant="danger" />
      </>,
    );

    expect(screen.getByRole("button", { name: "Strength help" })).toBeInTheDocument();
    expect(screen.getByRole("tooltip")).toHaveTextContent("NovelAI guidance strength");
    const slider = screen.getByRole("slider", { name: "Strength" });
    expect(slider).toHaveAttribute("min", "0");
    expect(slider).toHaveAttribute("max", "1");
    expect(slider).toHaveAttribute("step", "0.01");
    fireEvent.change(slider, { target: { value: "0.75" } });
    fireEvent.pointerUp(slider);
    expect(onChange).toHaveBeenCalledWith(0.75);
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Remove" })).toHaveClass("size-8");
  });
});
