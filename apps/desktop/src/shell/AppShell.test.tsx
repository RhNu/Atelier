import { render, screen } from "@testing-library/react";

import { AppShell } from "./AppShell";

const openWorkspaceStatus = { root: "D:/atelier", schema_version: 4, locked: true };

describe("AppShell", () => {
  it("shows a workspace gate when no workspace is open", () => {
    render(
      <AppShell
        workspaceStatus={null}
        workspacePending={false}
        workspaceErrorCode="workspace_not_open"
      />,
    );

    expect(screen.getByRole("heading", { name: "Open an Atelier workspace" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open workspace" })).toBeInTheDocument();
  });

  it("renders the routed workbench navigation when a workspace is open", () => {
    render(<AppShell workspaceStatus={openWorkspaceStatus} workspacePending={false} />);

    expect(screen.getByRole("navigation", { name: "Workspace sections" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Generate" })).toBeInTheDocument();
    expect(screen.getByText("D:/atelier")).toBeInTheDocument();
  });
});
