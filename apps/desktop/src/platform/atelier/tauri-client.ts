import { invoke } from "@tauri-apps/api/core";

import type { ErrorEnvelopeDto } from "@/types";

import { describeError, frontendLogger } from "../../app/logger";
import type { AtelierCommandName } from "./commands";

export class AtelierCommandError extends Error {
  code: string;
  details: unknown;

  constructor(payload: { code: string; message: string; details?: unknown }) {
    super(payload.message);
    this.name = "AtelierCommandError";
    this.code = payload.code;
    this.details = payload.details ?? null;
  }
}

export async function invokeAtelierCommand<T>(
  command: AtelierCommandName,
  args?: Record<string, unknown>,
): Promise<T> {
  frontendLogger.debug("Tauri command started", { command });
  try {
    const result = await invoke<T>(command, args);
    frontendLogger.debug("Tauri command completed", { command });
    return result;
  } catch (error) {
    const normalized = normalizeCommandError(error);
    frontendLogger.error("Tauri command failed", {
      command,
      error: describeError(normalized),
    });
    throw normalized;
  }
}

export function normalizeCommandError(error: unknown): AtelierCommandError {
  if (error instanceof AtelierCommandError) {
    return error;
  }

  if (isErrorEnvelope(error)) {
    return new AtelierCommandError({
      code: error.code,
      message: error.message,
      details: error.details,
    });
  }

  if (error instanceof Error) {
    return new AtelierCommandError({
      code: "tauri_invoke_error",
      message: error.message,
    });
  }

  return new AtelierCommandError({
    code: "tauri_invoke_error",
    message: typeof error === "string" ? error : "Tauri command failed",
    details: error,
  });
}

function isErrorEnvelope(value: unknown): value is ErrorEnvelopeDto {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as { code: unknown }).code === "string" &&
    typeof (value as { message: unknown }).message === "string"
  );
}
