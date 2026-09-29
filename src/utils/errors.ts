import type { AppError } from "../types/errors";

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).kind === "string" &&
    typeof (value as AppError).message === "string"
  );
}

/** Normalizes anything thrown by `invoke` (or elsewhere) into an AppError. */
export function toAppError(error: unknown): AppError {
  if (isAppError(error)) {
    return {
      kind: error.kind,
      message: error.message,
      detail: error.detail ?? null,
      params: error.params ?? {},
    };
  }
  if (error instanceof Error) {
    return { kind: "internal", message: error.message, detail: error.stack ?? null };
  }
  return { kind: "internal", message: String(error), detail: null };
}

/** Errors that suggest the camera is no longer reachable. */
export function isConnectionLoss(error: AppError): boolean {
  return error.kind === "unreachable" || error.kind === "timeout";
}
