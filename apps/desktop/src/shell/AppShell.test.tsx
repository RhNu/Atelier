import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { AppShell } from "./AppShell";

const openWorkspaceStatus = { root: "D:/atelier", schema_version: 4, locked: true };
const restoreFailure = {
  root: "D:/missing-atelier",
  error: { code: "storage", message: "workspace is missing", details: null },
};

describe("AppShell", () => {
  it("shows a workspace gate when no workspace is open", () => {
    render(<AppShell workspaceStatus={null} workspacePending={false} />);

    expect(screen.getByRole("heading", { name: "Open an Atelier workspace" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open workspace" })).toBeInTheDocument();
  });

  it("shows a recoverable error when the last workspace cannot be restored", async () => {
    const user = userEvent.setup();
    const retry = vi.fn<() => void>();
    const choose = vi.fn<() => void>();
    render(
      <AppShell
        workspaceStatus={null}
        workspacePending={false}
        restoreFailure={restoreFailure}
        onRetryWorkspaceRestore={retry}
        onOpenWorkspace={choose}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Couldn’t reopen your last workspace" }),
    ).toBeInTheDocument();
    expect(screen.getByText("D:/missing-atelier")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Retry" }));
    await user.click(screen.getByRole("button", { name: "Choose another workspace" }));
    expect(retry).toHaveBeenCalledTimes(1);
    expect(choose).toHaveBeenCalledTimes(1);
  });

  it("renders the routed workbench navigation when a workspace is open", () => {
    render(<AppShell workspaceStatus={openWorkspaceStatus} workspacePending={false} />);

    expect(screen.getByRole("navigation", { name: "Workspace sections" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Generate" })).toBeInTheDocument();
    expect(screen.queryByText("D:/atelier")).not.toBeInTheDocument();
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
