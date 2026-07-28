import { isTauri } from "@tauri-apps/api/core";
import {
  debug as writeTauriDebug,
  error as writeTauriError,
  info as writeTauriInfo,
  warn as writeTauriWarn,
  type LogOptions as TauriLogOptions,
} from "@tauri-apps/plugin-log";

export type LogContext = Readonly<Record<string, unknown>>;
export type LogLevel = "debug" | "info" | "warn" | "error";

type ConsoleMethod = (...data: unknown[]) => void;
type TauriLogMethod = (message: string, options?: TauriLogOptions) => Promise<void>;

const LOG_PREFIX = "[Atelier]";
const MAX_CONTEXT_VALUE_LENGTH = 8_000;
const tauriLogMethods: Readonly<Record<LogLevel, TauriLogMethod>> = {
  debug: writeTauriDebug,
  info: writeTauriInfo,
  warn: writeTauriWarn,
  error: writeTauriError,
};
let tauriForwardingEnabled = isTauri();
let tauriForwardingFailureReported = false;

export const frontendLogger = {
  debug(message: string, context?: LogContext): void {
    write("debug", message, context);
  },
  info(message: string, context?: LogContext): void {
    write("info", message, context);
  },
  warn(message: string, context?: LogContext): void {
    write("warn", message, context);
  },
  error(message: string, context?: LogContext): void {
    write("error", message, context);
  },
};

export function reportBackgroundPromise(
  promise: Promise<unknown>,
  operation: string,
  context?: LogContext,
): void {
  void promise.catch((error: unknown) => {
    frontendLogger.warn(`${operation} failed in background`, {
      ...context,
      error: describeError(error),
    });
  });
}

export async function runLoggedAction<T>(
  operation: string,
  action: () => Promise<T>,
  context?: LogContext,
): Promise<T> {
  frontendLogger.info(`${operation} started`, context);
  try {
    const result = await action();
    frontendLogger.info(`${operation} completed`, context);
    return result;
  } catch (error: unknown) {
    frontendLogger.error(`${operation} failed`, {
      ...context,
      error: describeError(error),
    });
    throw error;
  }
}

export function installGlobalErrorHandlers(): () => void {
  if (typeof window === "undefined") {
    return () => undefined;
  }

  const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    frontendLogger.error("Unhandled frontend promise rejection", {
      error: describeError(event.reason),
    });
  };
  const handleError = (event: ErrorEvent) => {
    frontendLogger.error("Unhandled frontend error", {
      error: describeError(event.error ?? event.message),
      filename: event.filename || undefined,
      line: event.lineno || undefined,
      column: event.colno || undefined,
    });
  };

  window.addEventListener("unhandledrejection", handleUnhandledRejection);
  window.addEventListener("error", handleError);
  return () => {
    window.removeEventListener("unhandledrejection", handleUnhandledRejection);
    window.removeEventListener("error", handleError);
  };
}

export function describeError(error: unknown): Record<string, unknown> {
  if (error instanceof Error) {
    const details = "details" in error ? error.details : undefined;
    const code = "code" in error ? error.code : undefined;
    return {
      name: error.name,
      message: error.message,
      ...(code !== undefined ? { code } : {}),
      ...(details !== undefined ? { details } : {}),
      ...(error.stack ? { stack: error.stack } : {}),
    };
  }

  return { value: error };
}

function write(level: LogLevel, message: string, context?: LogContext): void {
  const method: ConsoleMethod = console[level];
  const prefixedMessage = `${LOG_PREFIX} ${message}`;
  if (context && Object.keys(context).length > 0) {
    method(prefixedMessage, context);
  } else {
    method(prefixedMessage);
  }

  if (!tauriForwardingEnabled) {
    return;
  }
  void tauriLogMethods[level](prefixedMessage, toTauriLogOptions(context)).catch(
    handleTauriForwardingFailure,
  );
}

function toTauriLogOptions(context?: LogContext): TauriLogOptions | undefined {
  if (!context || Object.keys(context).length === 0) {
    return undefined;
  }

  return {
    keyValues: Object.fromEntries(
      Object.entries(context).map(([key, value]) => [key, serializeLogValue(value)]),
    ),
  };
}

function serializeLogValue(value: unknown): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (typeof value === "string") {
    return truncate(value);
  }
  if (value === null || typeof value !== "object") {
    return truncate(describeUnserializableValue(value));
  }

  const seen = new WeakSet<object>();
  try {
    const serialized = JSON.stringify(value, (_key, nestedValue: unknown) => {
      if (typeof nestedValue === "bigint") {
        return nestedValue.toString();
      }
      if (nestedValue instanceof Error) {
        return describeError(nestedValue);
      }
      if (typeof nestedValue === "object" && nestedValue !== null) {
        if (seen.has(nestedValue)) {
          return "[Circular]";
        }
        seen.add(nestedValue);
      }
      return nestedValue;
    });
    return truncate(serialized ?? describeUnserializableValue(value));
  } catch {
    return truncate(describeUnserializableValue(value));
  }
}

function describeUnserializableValue(value: unknown): string {
  if (value === null) {
    return "null";
  }
  switch (typeof value) {
    case "number":
    case "bigint":
      return value.toString();
    case "boolean":
      return value ? "true" : "false";
    case "symbol":
      return value.description ? `Symbol(${value.description})` : "Symbol";
    case "function":
      return value.name ? `[Function ${value.name}]` : "[Function]";
    case "object":
      return "[Unserializable object]";
    case "string":
      return value;
    case "undefined":
      return "undefined";
  }
  return "[Unknown]";
}

function truncate(value: string): string {
  if (value.length <= MAX_CONTEXT_VALUE_LENGTH) {
    return value;
  }
  return `${value.slice(0, MAX_CONTEXT_VALUE_LENGTH)}…`;
}

function handleTauriForwardingFailure(error: unknown): void {
  tauriForwardingEnabled = false;
  if (tauriForwardingFailureReported) {
    return;
  }
  tauriForwardingFailureReported = true;
  console.error(`${LOG_PREFIX} Frontend log forwarding to Tauri was disabled`, error);
}
