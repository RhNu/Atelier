import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";

import { SeedInput } from "./SeedInput";

function SeedHarness() {
  const [seed, setSeed] = useState(0);
  return <SeedInput label="Seed" value={seed} randomPlaceholder="Random" onChange={setSeed} />;
}

describe("SeedInput", () => {
  it("uses an empty value for random and accepts only safe positive integers", () => {
    render(<SeedHarness />);
    const input = screen.getByLabelText("Seed");

    expect(input).toHaveValue(null);
    expect(input).toHaveAttribute("min", "1");
    expect(input).toHaveAttribute("max", String(Number.MAX_SAFE_INTEGER));

    fireEvent.change(input, { target: { value: "42" } });
    expect(input).toHaveValue(42);

    fireEvent.change(input, { target: { value: "-1" } });
    expect(input).toHaveValue(42);
    fireEvent.change(input, { target: { value: "1.5" } });
    expect(input).toHaveValue(42);
    fireEvent.change(input, { target: { value: String(Number.MAX_SAFE_INTEGER + 1) } });
    expect(input).toHaveValue(42);

    fireEvent.change(input, { target: { value: "" } });
    expect(input).toHaveValue(null);
  });
});
