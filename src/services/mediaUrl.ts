import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * URL a `<video>` or `<img>` can load a camera file from. The `virb`
 * protocol is served by the backend (src-tauri/src/media_protocol.rs),
 * which forwards the request to the connected camera.
 */
export function cameraMediaUrl(cameraUrl: string): string {
  return convertFileSrc(cameraUrl, "virb");
}
