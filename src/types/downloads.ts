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
