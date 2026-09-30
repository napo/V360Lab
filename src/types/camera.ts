// TypeScript mirrors of the Rust models in src-tauri/src/camera/models.rs.
// Optional camera fields are `null` when the camera did not report them.

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export type CameraKind = "garmin-virb-360" | "mock";

export interface DeviceInfo {
  model: string | null;
  firmware: string | null;
  deviceId: string | null;
  partNumber: string | null;
  deviceType: string | null;
  raw: JsonValue;
}

export type RecordingState = "idle" | "recording" | "unknown";

export interface CameraStatus {
  recordingState: RecordingState;
  mode: string | null;
  batteryLevel: number | null;
  batteryChargingState: string | null;
  storageTotalBytes: number | null;
  storageAvailableBytes: number | null;
  recordingTimeSecs: number | null;
  recordingTimeRemainingSecs: number | null;
  gpsLatitude: number | null;
  gpsLongitude: number | null;
  raw: JsonValue;
}

export interface CameraFeature {
  key: string;
  label: string | null;
  value: JsonValue;
  options: JsonValue[];
  optionSummaries: string[];
  enabled: boolean | null;
  featureType: JsonValue;
  raw: JsonValue;
}

export interface FeatureList {
  features: CameraFeature[];
  raw: JsonValue;
}

export type MediaType = "video" | "photo" | "other";

export interface MediaItem {
  id: string;
  name: string;
  mediaType: MediaType;
  mediaTypeRaw: string | null;
  timestamp: number | null;
  dateTime: string | null;
  durationSecs: number | null;
  fileSizeBytes: number | null;
  lensMode: string | null;
  url: string | null;
  thumbnailUrl: string | null;
  lowResUrl: string | null;
  fitUrl: string | null;
  hasFit: boolean;
  raw: JsonValue;
}

export interface CommandAck {
  command: string;
  raw: JsonValue;
}

export interface ConnectionInfo {
  kind: CameraKind;
  address: string;
  deviceInfo: DeviceInfo;
  status: CameraStatus | null;
}

export interface ActiveConnection {
  kind: CameraKind;
  address: string;
}

/** A camera found by `discover_cameras`. */
export interface DiscoveredCamera {
  address: string;
  model: string | null;
  firmware: string | null;
  deviceId: string | null;
}

export type CameraAction =
  | "startRecording"
  | "stopRecording"
  | "snapPicture"
  | "stopStillRecording";

/** Wi-Fi security values as the VIRB names them. */
export type WifiSecurity = "WPA2" | "WPA" | "WEP" | "Open";

export const WIFI_SECURITY_TYPES: WifiSecurity[] = ["WPA2", "WPA", "WEP", "Open"];

export interface WifiNetwork {
  ssid: string;
  /** Null when the camera reports an unknown value (see `securityRaw`). */
  security: WifiSecurity | null;
  securityRaw: string | null;
}

export interface WifiNetworks {
  /** Name of the network the camera creates itself. */
  accessPointSsid: string | null;
  /** Networks saved on the camera, which it joins automatically. */
  configured: WifiNetwork[];
  /** Networks the camera can see now. */
  scanned: WifiNetwork[];
}

export interface WifiSwitch {
  /** False when the camera left before answering (it is most likely switching). */
  confirmed: boolean;
}
