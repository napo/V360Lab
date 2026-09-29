import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { MediaItem } from "../types/camera";
import type { DownloadOptions, DownloadProgress, DownloadReport } from "../types/downloads";
import { call, isTauri } from "./backend";

const PROGRESS_EVENT = "download-progress";

export const downloadService = {
  downloadMedia: (item: MediaItem, options: DownloadOptions) =>
    call<DownloadReport>("download_media", {
      item,
      includeFit: options.includeFit,
      includeThumbnail: options.includeThumbnail,
    }),
  downloadFit: (item: MediaItem) => call<DownloadReport>("download_fit", { item }),

  /** Subscribes to backend progress events; returns an unsubscribe function. */
  onProgress: async (handler: (progress: DownloadProgress) => void): Promise<UnlistenFn> => {
    if (!isTauri()) return () => {};
    return listen<DownloadProgress>(PROGRESS_EVENT, (event) => handler(event.payload));
  },
};
