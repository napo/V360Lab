import { invoke } from "@tauri-apps/api/core";
import { toAppError } from "../utils/errors";

/** True when running inside the Tauri webview (not a plain browser tab). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Invokes a Rust command. All camera communication goes through here:
 * the UI never talks to the camera over the network itself.
 * Rejections are always normalized to an `AppError`.
 */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw toAppError({
      kind: "backendUnavailable",
      message: "The V360Lab backend is not available. Start the app with `npm run tauri dev`.",
      detail: null,
    });
  }
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}
