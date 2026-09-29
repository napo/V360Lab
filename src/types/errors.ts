/** Error payload produced by the Rust backend (see src-tauri/src/error.rs). */
export interface AppError {
  /** Machine-readable category, e.g. "unreachable", "timeout", "http". */
  kind: string;
  /** Readable message for the user. */
  message: string;
  /** Technical details, shown only in debug mode. */
  detail: string | null;
}
