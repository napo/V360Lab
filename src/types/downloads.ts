import type { AppError } from "./errors";

// Mirrors src-tauri/src/downloads/mod.rs.

export type FileKind = "media" | "telemetry" | "thumbnail" | "metadata";

export interface DownloadedFile {
  kind: FileKind;
  path: string;
  bytes: number;
  skipped: boolean;
}

/** Non-fatal problem; `code` is translated, `detail` is technical English. */
export interface DownloadWarning {
  code: string;
  detail: string;
}

export interface DownloadReport {
  itemId: string;
  directory: string;
  files: DownloadedFile[];
  warnings: DownloadWarning[];
}

/** Result of deleting media on the camera (see src-tauri/src/library.rs). */
export interface DeleteReport {
  deleted: string[];
  failed: Array<{ itemId: string; name: string; error: AppError }>;
  /** False when the media list could not be re-read to confirm. */
  verified: boolean;
}

export interface DownloadProgress {
  itemId: string;
  kind: FileKind;
  fileName: string;
  receivedBytes: number;
  totalBytes: number | null;
}

export interface DownloadOptions {
  includeFit: boolean;
  includeThumbnail: boolean;
}
