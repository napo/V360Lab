import type {
  ActiveConnection,
  CameraAction,
  CameraStatus,
  CommandAck,
  ConnectionInfo,
  DeviceInfo,
  FeatureList,
  MediaItem,
} from "../types/camera";
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
  connect: (address: string, mock: boolean) =>
    call<ConnectionInfo>("connect_camera", { address, mock }),
  disconnect: () => call<void>("disconnect_camera"),
  activeConnection: () => call<ActiveConnection | null>("get_active_connection"),
  deviceInfo: shared(() => call<DeviceInfo>("get_device_info")),
  status: shared(() => call<CameraStatus>("get_camera_status")),
  features: shared(() => call<FeatureList>("get_camera_features")),
  mediaList: shared(() => call<MediaItem[]>("get_media_list")),
  thumbnail: (url: string) => call<string>("fetch_thumbnail", { url }),
  runAction: (action: CameraAction) => {
    const commands: Record<CameraAction, string> = {
      startRecording: "start_recording",
      stopRecording: "stop_recording",
      snapPicture: "snap_picture",
    };
    return call<CommandAck>(commands[action]);
  },
};
