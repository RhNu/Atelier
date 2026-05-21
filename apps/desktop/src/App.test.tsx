import { render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { createAtelierQueryClient } from "./app/query-client";

vi.mock("./features/workspace/useWorkspaceStatus", () => ({
  useWorkspaceStatus: () => ({
    workspaceStatus: { root: "D:/atelier", schema_version: 4, locked: false },
    workspacePending: false,
    workspaceErrorCode: undefined,
    workspaceErrorMessage: undefined,
    openWorkspace: vi.fn(),
    closeWorkspace: vi.fn(),
    openingWorkspace: false,
    closingWorkspace: false,
  }),
}));

describe("App", () => {
  it("renders the routed desktop shell", async () => {
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <App />
      </QueryClientProvider>,
    );

    expect(
      await screen.findByRole("navigation", { name: "Workspace sections" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Generate" })).toBeInTheDocument();
  });
});
