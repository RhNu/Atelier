import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { i18n } from "@/i18n";

import { AppShell } from "./AppShell";

const openWorkspaceStatus = { root: "D:/atelier", schema_version: 4, locked: true };
const restoreFailure = {
  root: "D:/missing-atelier",
  error: { code: "storage", message: "workspace is missing", details: null },
};

describe("AppShell", () => {
  it("shows a workspace gate when no workspace is open", () => {
    render(<AppShell workspaceStatus={null} workspacePending={false} />);

    expect(screen.getByRole("heading", { name: i18n.t("shell:openTitle") })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: i18n.t("shell:openWorkspace") })).toBeInTheDocument();
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

    expect(screen.getByRole("heading", { name: i18n.t("shell:reopenFailed") })).toBeInTheDocument();
    expect(screen.getByText("D:/missing-atelier")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: i18n.t("common:retry") }));
    await user.click(screen.getByRole("button", { name: i18n.t("shell:chooseAnotherWorkspace") }));
    expect(retry).toHaveBeenCalledTimes(1);
    expect(choose).toHaveBeenCalledTimes(1);
  });

  it("renders the routed workbench navigation when a workspace is open", () => {
    render(<AppShell workspaceStatus={openWorkspaceStatus} workspacePending={false} />);

    expect(
      screen.getByRole("navigation", { name: i18n.t("shell:workspaceSections") }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: i18n.t("shell:nav.generate") })).toBeInTheDocument();
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

    await user.click(screen.getByRole("link", { name: i18n.t("shell:nav.gallery") }));

    expect(onNavigate).toHaveBeenCalledWith("/gallery");
  });
});
