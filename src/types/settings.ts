// Mirrors src-tauri/src/settings.rs and the settings commands.

export interface Settings {
  lastCameraAddress: string | null;
  downloadDirectory: string | null;
  mockMode: boolean;
  debugMode: boolean;
  statusPollIntervalSecs: number;
  /** "en" | "it"; null follows the system language. */
  language: string | null;
  /** YOLO model (ONNX file) for object detection. */
  detectionModel: string | null;
  /** Look for a newer release on GitHub when the app starts. */
  checkUpdates: boolean;
}

/** Result of the check for a newer release. */
export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  /** Release notes (Markdown). */
  notes: string;
  pageUrl: string;
  apkUrl: string | null;
  platform: "android" | "desktop";
}

export interface SettingsView {
  settings: Settings;
  effectiveDownloadDirectory: string;
  defaultCameraAddress: string;
}

export interface AppInfo {
  name: string;
  version: string;
  debugBuild: boolean;
}
