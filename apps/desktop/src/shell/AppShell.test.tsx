import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

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
    expect(screen.queryByRole("button", { name: "Collapse panels" })).not.toBeInTheDocument();
  });

  it("routes navigation clicks through the shell callback", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn<(to: string) => void>();
    render(
      <AppShell
        workspaceStatus={openWorkspaceStatus}
        workspacePending={false}
        onNavigate={onNavigate}
      />,
    );

    await user.click(screen.getByRole("link", { name: "Gallery" }));

    expect(onNavigate).toHaveBeenCalledWith("/gallery");
  });
});
