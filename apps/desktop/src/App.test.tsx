import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the workspace scaffold", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "NAI Atelier" })).toBeInTheDocument();
    expect(screen.getByText("Vite React TS")).toBeInTheDocument();
    expect(screen.getByText("Tauri v2")).toBeInTheDocument();
  });
});
