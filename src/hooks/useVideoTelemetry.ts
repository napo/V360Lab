import { useEffect, useState } from "react";
import { cameraService } from "../services/cameraService";
import type { MediaItem, VideoTelemetry } from "../types/camera";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";

export interface TelemetryState {
  data: VideoTelemetry | null;
  error: AppError | null;
  loading: boolean;
}

/** The FIT telemetry of a video (none when `item` is null). */
export function useVideoTelemetry(item: MediaItem | null): TelemetryState {
  const [state, setState] = useState<TelemetryState>({ data: null, error: null, loading: false });

  useEffect(() => {
    if (!item) {
      setState({ data: null, error: null, loading: false });
      return;
    }
    let cancelled = false;
    setState({ data: null, error: null, loading: true });
    cameraService
      .telemetry(item)
      .then((data) => !cancelled && setState({ data, error: null, loading: false }))
      .catch((e) => !cancelled && setState({ data: null, error: toAppError(e), loading: false }));
    return () => {
      cancelled = true;
    };
  }, [item]);

  return state;
}
