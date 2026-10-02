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
  /** Accessories connected to the camera (null: not reported). */
  bluetoothHeadset: boolean | null;
  bluetoothSensor: boolean | null;
  antSensor: boolean | null;
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
  /** Marked as favourite on the camera; null when not reported. */
  favorite: boolean | null;
  raw: JsonValue;
}

/** A sensor paired with the camera (heart rate, cadence, …). */
export interface SensorInfo {
  name: string;
  /** How it is connected, e.g. `ANT` or `LOCAL`. */
  sensorType: string | null;
  found: boolean | null;
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

export interface TelemetrySample {
  /** UTC milliseconds since the Unix epoch. */
  timestampMs: number;
  latitude: number | null;
  longitude: number | null;
  altitudeM: number | null;
  speedMps: number | null;
  headingDeg: number | null;
  heartRate: number | null;
}

export interface TrackSummary {
  durationSecs: number;
  distanceM: number;
  maxSpeedMps: number | null;
  avgSpeedMps: number | null;
  minAltitudeM: number | null;
  maxAltitudeM: number | null;
  elevationGainM: number;
  sampleCount: number;
  hasPosition: boolean;
  /** Positions dropped as GPS jumps. */
  droppedPositions: number;
}

/** The GPS track of a video, aligned with its timeline. */
export interface VideoTelemetry {
  /** UTC milliseconds of the first video frame. */
  videoStartMs: number;
  startFromCameraEvent: boolean;
  samples: TelemetrySample[];
  summary: TrackSummary;
  /** Camera tilt during the video; empty when the FIT file has none. */
  accelerometer: AccelSample[];
}

/** Accelerometer reading (camera frame, about 1 g pointing up at rest). */
export interface AccelSample {
  timestampMs: number;
  x: number;
  y: number;
  z: number;
}

export interface DetectionProgress {
  done: number;
  total: number;
  detections: number;
}

export interface DetectionReport {
  frames: number;
  detections: number;
  /** [class, count], most frequent first. */
  byClass: Array<[string, number]>;
  /** Path of detections.geojson. */
  path: string;
  cancelled: boolean;
}

/** A detection model V360Lab can download. */
export interface ModelStatus {
  id: string;
  name: string;
  fileName: string;
  url: string;
  sizeBytes: number;
  sha256: string;
  classes: number;
  license: string;
  /** Path of the downloaded file, if present. */
  installedPath: string | null;
  /** It is the model object detection uses now. */
  active: boolean;
}

export interface DownloadCheck {
  sizeBytes: number;
  remoteSizeBytes: number | null;
  freeBytes: number | null;
  enoughSpace: boolean;
}

export interface ModelDownloadProgress {
  received: number;
  total: number;
}

/** How fast this device runs the detection model. */
export interface DetectionBenchmark {
  loadMs: number;
  /** One view, median of a few runs. */
  viewMs: number;
  viewsPerFrame: number;
  /** Estimated time per 360° frame. */
  frameMs: number;
}
