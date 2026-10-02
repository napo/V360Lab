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
