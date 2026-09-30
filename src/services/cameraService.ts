import type {
  ActiveConnection,
  CameraAction,
  CameraStatus,
  CommandAck,
  ConnectionInfo,
  DeviceInfo,
  DiscoveredCamera,
  FeatureList,
  MediaItem,
  WifiNetworks,
  WifiSecurity,
  WifiSwitch,
} from "../types/camera";
import type { DeleteReport } from "../types/downloads";
import { call } from "./backend";

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
