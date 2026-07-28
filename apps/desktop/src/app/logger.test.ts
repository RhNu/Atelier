import { afterEach, describe, expect, it, vi } from "vitest";

const tauriLogMocks = vi.hoisted(() => ({
  debug: vi.fn<(message: string, options?: unknown) => Promise<void>>().mockResolvedValue(),
  error: vi.fn<(message: string, options?: unknown) => Promise<void>>().mockResolvedValue(),
  info: vi.fn<(message: string, options?: unknown) => Promise<void>>().mockResolvedValue(),
  warn: vi.fn<(message: string, options?: unknown) => Promise<void>>().mockResolvedValue(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));
vi.mock("@tauri-apps/plugin-log", () => tauriLogMocks);

afterEach(() => {
  vi.restoreAllMocks();
  for (const mock of Object.values(tauriLogMocks)) {
    mock.mockClear();
  }
});

describe("frontendLogger", () => {
  it("keeps the console record and forwards a serializable copy to Tauri", async () => {
    vi.resetModules();
    const { frontendLogger } = await import("./logger");
    const consoleInfo = vi.spyOn(console, "info").mockImplementation(() => undefined);
    const circular: { self?: unknown } = {};
    circular.self = circular;

    frontendLogger.info("Generation prepared", {
      attempts: 2n,
      circular,
      workspace: "D:\\Atelier",
    });

    expect(consoleInfo).toHaveBeenCalledWith("[Atelier] Generation prepared", {
      attempts: 2n,
      circular,
      workspace: "D:\\Atelier",
    });
    expect(tauriLogMocks.info).toHaveBeenCalledWith("[Atelier] Generation prepared", {
      keyValues: {
        attempts: "2",
        circular: '{"self":"[Circular]"}',
        workspace: "D:\\Atelier",
      },
    });
  });
});
