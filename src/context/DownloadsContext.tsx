import { createContext, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { downloadService } from "../services/downloadService";
import type { MediaItem } from "../types/camera";
import type { DownloadOptions, DownloadProgress, DownloadReport } from "../types/downloads";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";

export type DownloadTarget = "media" | "fit";

export type DownloadState =
  | { status: "downloading"; target: DownloadTarget; progress: DownloadProgress | null }
  | { status: "done"; target: DownloadTarget; report: DownloadReport }
  | { status: "error"; target: DownloadTarget; error: AppError };

export interface DownloadsContextValue {
  /** Download state per media item id. */
  downloads: Record<string, DownloadState>;
  start: (item: MediaItem, target: DownloadTarget, options: DownloadOptions) => Promise<void>;
}

export const DownloadsContext = createContext<DownloadsContextValue | null>(null);

/** Kept at the app root so downloads continue to be tracked across pages. */
export function DownloadsProvider({ children }: { children: ReactNode }) {
  const [downloads, setDownloads] = useState<Record<string, DownloadState>>({});

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void downloadService
      .onProgress((progress) =>
        setDownloads((current) => {
          const entry = current[progress.itemId];
          if (entry?.status !== "downloading") return current;
          return { ...current, [progress.itemId]: { ...entry, progress } };
        }),
      )
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const start = useCallback(
    async (item: MediaItem, target: DownloadTarget, options: DownloadOptions) => {
      const set = (state: DownloadState) =>
        setDownloads((current) => ({ ...current, [item.id]: state }));
      set({ status: "downloading", target, progress: null });
      try {
        const report =
          target === "media"
            ? await downloadService.downloadMedia(item, options)
            : await downloadService.downloadFit(item);
        set({ status: "done", target, report });
      } catch (e) {
        set({ status: "error", target, error: toAppError(e) });
      }
    },
    [],
  );

  const value = useMemo(() => ({ downloads, start }), [downloads, start]);
  return <DownloadsContext.Provider value={value}>{children}</DownloadsContext.Provider>;
}
