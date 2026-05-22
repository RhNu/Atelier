import { invoke } from "@tauri-apps/api/core";

import { AtelierCommandError, invokeAtelierCommand } from "./tauri-client";

type InvokeFunction = typeof invoke;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn<InvokeFunction>(),
}));

const invokeMock = vi.mocked(invoke);

describe("invokeAtelierCommand", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("returns successful command payloads", async () => {
    invokeMock.mockResolvedValue({ root: "D:/atelier", locked: true, schema_version: 4 });

    await expect(invokeAtelierCommand("workspace_status")).resolves.toEqual({
      root: "D:/atelier",
      locked: true,
      schema_version: 4,
    });
  });

  it("normalizes command error envelopes", async () => {
    invokeMock.mockRejectedValue({
      code: "workspace_not_open",
      message: "workspace is not open",
      details: { reason: "missing" },
    });

    await expect(invokeAtelierCommand("workspace_status")).rejects.toMatchObject({
      name: "AtelierCommandError",
      code: "workspace_not_open",
      message: "workspace is not open",
      details: { reason: "missing" },
    } satisfies Partial<AtelierCommandError>);
  });
});
