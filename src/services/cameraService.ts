import type {
  ActiveConnection,
  CameraAction,
  CameraStatus,
  CommandAck,
  ConnectionInfo,
  DetectionBenchmark,
  DetectionProgress,
  DetectionReport,
  DownloadCheck,
  ModelDownloadProgress,
  ModelStatus,
  DeviceInfo,
  DiscoveredCamera,
  FeatureList,
  MediaItem,
  SensorInfo,
  VideoTelemetry,
  WifiNetworks,
  WifiSecurity,
  WifiSwitch,
} from "../types/camera";
import type { DeleteReport } from "../types/downloads";
import type { SettingsView } from "../types/settings";
import type { FrameLocation } from "../utils/frames";
import { Channel } from "@tauri-apps/api/core";
import { call, callRaw } from "./backend";

/**
 * Wraps a read-only request so concurrent callers share one in-flight call.
 * The camera is slow (mediaList can take seconds), and duplicate requests
 * come from overlapping polls/refreshes and from React StrictMode in dev.
 */
function shared<T>(request: () => Promise<T>): () => Promise<T> {
  let pending: Promise<T> | null = null;
  return () => {
    pending ??= request().finally(() => {
      pending = null;
    });
    return pending;
  };
}

/** Typed wrappers around the camera-related Tauri commands. */
export const cameraService = {
  connect: (address: string, mock: boolean, activityId: string) =>
    call<ConnectionInfo>("connect_camera", { address, mock, activityId }),
  discover: (activityId: string) => call<DiscoveredCamera[]>("discover_cameras", { activityId }),
  disconnect: () => call<void>("disconnect_camera"),
  activeConnection: () => call<ActiveConnection | null>("get_active_connection"),
  deviceInfo: shared(() => call<DeviceInfo>("get_device_info")),
  status: shared(() => call<CameraStatus>("get_camera_status")),
  features: shared(() => call<FeatureList>("get_camera_features")),
  mediaList: shared(() => call<MediaItem[]>("get_media_list")),
  updateFeature: (key: string, value: string) =>
    call<FeatureList>("update_feature", { key, value }),
  deleteMedia: (items: MediaItem[], activityId: string) =>
    call<DeleteReport>("delete_media", { items, activityId }),
  thumbnail: (url: string) => call<string>("fetch_thumbnail", { url }),
  supportedCommands: shared(() => call<string[] | null>("get_supported_commands")),
  locate: (on: boolean) => call<CommandAck>("locate_camera", { on }),
  sensors: () => call<SensorInfo[]>("get_sensors"),
  mediaDirectories: () => call<string[]>("get_media_directories"),
  standby: () => call<CommandAck>("standby_camera"),
  telemetry: (item: MediaItem) => call<VideoTelemetry>("get_media_telemetry", { item }),
  /** Writes the GPS track next to the downloads; resolves to the file path. */
  exportTrack: (item: MediaItem, format: "gpx" | "geojson") => call<string>("export_track", { item, format }),
  /** Saves an extracted video frame (JPEG) with GPS tags; resolves to its path. */
  saveFrame: (
    jpeg: Uint8Array,
    frame: { name: string; timestamp: number | null; index: number; location: FrameLocation; spherical: boolean },
  ) =>
    callRaw<string>("save_frame", jpeg, {
      "x-v360lab-frame": encodeURIComponent(JSON.stringify(frame)),
    }),
  writeFramesIndex: (
    item: MediaItem,
    frames: Array<{ file: string; videoSeconds: number; location: FrameLocation }>,
  ) => call<string>("write_frames_index", { item, frames }),
  /** Finds objects in the video's extracted frames with the chosen model. */
  detectObjects: (item: MediaItem, minConfidence: number, onProgress: (progress: DetectionProgress) => void) => {
    const progress = new Channel<DetectionProgress>();
    progress.onmessage = onProgress;
    return call<DetectionReport>("detect_objects", { item, minConfidence, progress });
  },
  cancelDetection: () => call<void>("cancel_detection"),
  benchmarkDetection: () => call<DetectionBenchmark>("benchmark_detection"),
  listModels: () => call<ModelStatus[]>("list_models"),
  checkModelDownload: (id: string) => call<DownloadCheck>("check_model_download", { id }),
  /** Downloads a model and makes it the active one; resolves to the new settings. */
  downloadModel: (id: string, onProgress: (progress: ModelDownloadProgress) => void) => {
    const progress = new Channel<ModelDownloadProgress>();
    progress.onmessage = onProgress;
    return call<SettingsView>("download_model", { id, progress });
  },
  setFavorite: (item: MediaItem, favorite: boolean) =>
    call<MediaItem>("set_media_favorite", { item, favorite }),
  wifiNetworks: shared(() => call<WifiNetworks>("get_wifi_networks")),
  addWifiNetwork: (ssid: string, security: WifiSecurity, password: string) =>
    call<CommandAck>("add_wifi_network", { ssid, security, password }),
  connectWifiNetwork: (ssid: string) => call<WifiSwitch>("connect_wifi_network", { ssid }),
  removeWifiNetwork: (ssid: string) => call<CommandAck>("remove_wifi_network", { ssid }),
  runAction: (action: CameraAction) => {
    const commands: Record<CameraAction, string> = {
      startRecording: "start_recording",
      stopRecording: "stop_recording",
      snapPicture: "snap_picture",
      stopStillRecording: "stop_still_recording",
    };
    return call<CommandAck>(commands[action]);
  },
};
