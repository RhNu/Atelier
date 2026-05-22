import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Sparkles } from "lucide-react";

import { AppButton } from "./AppButton";
import { AppIconButton } from "./AppIconButton";
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
});
