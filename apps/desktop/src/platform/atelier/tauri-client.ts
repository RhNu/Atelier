import { invoke } from "@tauri-apps/api/core";

import type { ErrorEnvelopeDto } from "../../types";
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
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeCommandError(error);
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
