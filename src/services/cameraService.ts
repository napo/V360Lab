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

/** Typed wrappers around the camera-related Tauri commands. */
export const cameraService = {
  connect: (address: string, mock: boolean) =>
    call<ConnectionInfo>("connect_camera", { address, mock }),
  disconnect: () => call<void>("disconnect_camera"),
  activeConnection: () => call<ActiveConnection | null>("get_active_connection"),
  deviceInfo: () => call<DeviceInfo>("get_device_info"),
  status: () => call<CameraStatus>("get_camera_status"),
  features: () => call<FeatureList>("get_camera_features"),
  mediaList: () => call<MediaItem[]>("get_media_list"),
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
