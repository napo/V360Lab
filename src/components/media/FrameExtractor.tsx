import { useEffect, useRef, useState } from "react";
import { useI18n } from "../../hooks/useI18n";
import { cameraService } from "../../services/cameraService";
import { cameraMediaUrl } from "../../services/mediaUrl";
import type { MediaItem, VideoTelemetry } from "../../types/camera";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { planFrames, type FrameSpacing } from "../../utils/frames";
import { ErrorBanner } from "../ErrorBanner";

const JPEG_QUALITY = 0.92;
/** Give up on a frame the video element cannot reach. */
const SEEK_TIMEOUT_MS = 20_000;

type Quality = "preview" | "original";
type State =
  | { status: "idle" }
  | { status: "running"; done: number; total: number }
  | { status: "done"; count: number; index: string }
  | { status: "cancelled"; count: number };

interface FrameExtractorProps {
  item: MediaItem;
  telemetry: VideoTelemetry;
}

/** Moves the video to `seconds` and resolves once that frame is shown. */
function seekTo(video: HTMLVideoElement, seconds: number): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      cleanup();
      reject(new Error(`could not reach ${seconds.toFixed(1)} s in the video`));
    }, SEEK_TIMEOUT_MS);
    const onSeeked = () => {
      cleanup();
      // Wait for the frame to be presented before reading it, but not
      // forever: some webviews do not report frames of an unseen video.
      const fallback = window.setTimeout(resolve, 300);
      const done = () => {
        window.clearTimeout(fallback);
        resolve();
      };
      if (typeof video.requestVideoFrameCallback === "function") video.requestVideoFrameCallback(done);
      else requestAnimationFrame(done);
    };
    const onError = () => {
      cleanup();
      reject(new Error("the video cannot be decoded on this device"));
    };
    const cleanup = () => {
      window.clearTimeout(timer);
      video.removeEventListener("seeked", onSeeked);
      video.removeEventListener("error", onError);
    };
    video.addEventListener("seeked", onSeeked);
    video.addEventListener("error", onError);
    video.currentTime = seconds;
  });
}

function waitForMetadata(video: HTMLVideoElement): Promise<void> {
  if (video.readyState >= 1) return Promise.resolve();
  return new Promise((resolve, reject) => {
    video.addEventListener("loadedmetadata", () => resolve(), { once: true });
    video.addEventListener("error", () => reject(new Error("the video cannot be decoded on this device")), {
      once: true,
    });
  });
}

/**
 * Extracts georeferenced frames from a video: every N metres travelled or
 * every N seconds, as JPEG files with EXIF GPS (and 360° panorama tags for
 * 360° videos) plus a `frames.geojson` index, next to the downloads.
 */
export function FrameExtractor({ item, telemetry }: FrameExtractorProps) {
  const { t } = useI18n();
  const [kind, setKind] = useState<FrameSpacing["kind"]>("distance");
  const [meters, setMeters] = useState(5);
  const [seconds, setSeconds] = useState(1);
  const [quality, setQuality] = useState<Quality>(item.url ? "original" : "preview");
  const [state, setState] = useState<State>({ status: "idle" });
  const [error, setError] = useState<AppError | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const cancelled = useRef(false);

  useEffect(
    () => () => {
      cancelled.current = true;
    },
    [],
  );

  const spacing: FrameSpacing = kind === "distance" ? { kind, meters } : { kind, seconds };
  const durationMs = (item.durationSecs ?? telemetry.summary.durationSecs) * 1000;
  const planned = planFrames(telemetry.samples, telemetry.videoStartMs, durationMs, spacing);
  const sourceUrl = quality === "preview" ? item.lowResUrl : item.url;
  const spherical = item.lensMode === "360";

  const extract = async () => {
    const video = videoRef.current;
    if (!video || !sourceUrl || planned.length === 0) return;
    cancelled.current = false;
    setError(null);
    setState({ status: "running", done: 0, total: planned.length });
    const saved: Array<{ file: string; videoSeconds: number; location: (typeof planned)[number]["location"] }> = [];
    try {
      video.src = cameraMediaUrl(sourceUrl);
      await waitForMetadata(video);
      // 360° frames are written as 2:1 equirectangular images, whatever the
      // proportions of the video (the camera's preview copy is 16:9).
      const width = video.videoWidth;
      const height = spherical ? Math.round(width / 2) : video.videoHeight;
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d");
      if (!context) throw new Error("canvas unavailable");

      for (const [index, frame] of planned.entries()) {
        if (cancelled.current) {
          setState({ status: "cancelled", count: saved.length });
          break;
        }
        await seekTo(video, Math.min(frame.videoSeconds, Math.max(video.duration - 0.05, 0)));
        context.drawImage(video, 0, 0, width, height);
        const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg", JPEG_QUALITY));
        if (!blob) throw new Error("the frame could not be encoded");
        const file = await cameraService.saveFrame(new Uint8Array(await blob.arrayBuffer()), {
          name: item.name,
          timestamp: item.timestamp,
          index: index + 1,
          location: frame.location,
          spherical,
        });
        saved.push({ file, videoSeconds: frame.videoSeconds, location: frame.location });
        setState({ status: "running", done: index + 1, total: planned.length });
      }
      if (saved.length > 0) {
        const index = await cameraService.writeFramesIndex(item, saved);
        if (!cancelled.current) setState({ status: "done", count: saved.length, index });
      }
    } catch (e) {
      setError(
        e instanceof DOMException && e.name === "SecurityError"
          ? toAppError(new Error("the webview does not allow reading the video frames"))
          : toAppError(e),
      );
      setState({ status: "idle" });
      if (saved.length > 0) void cameraService.writeFramesIndex(item, saved).catch(() => {});
    } finally {
      video.removeAttribute("src");
      video.load();
    }
  };

  const running = state.status === "running";

  return (
    <section className="frame-extractor" aria-label={t("frames.title")}>
      <h3>{t("frames.title")}</h3>
      <p className="muted small">{spherical ? t("frames.intro360") : t("frames.intro")}</p>
      <div className="frame-options">
        <select value={kind} disabled={running} onChange={(e) => setKind(e.target.value as FrameSpacing["kind"])}>
          <option value="distance">{t("frames.everyMeters")}</option>
          <option value="time">{t("frames.everySeconds")}</option>
        </select>
        {kind === "distance" ? (
          <input
            type="number"
            min={1}
            max={1000}
            value={meters}
            disabled={running}
            aria-label={t("frames.meters")}
            onChange={(e) => setMeters(Math.max(1, Number(e.target.value) || 1))}
          />
        ) : (
          <input
            type="number"
            min={0.2}
            max={600}
            step={0.1}
            value={seconds}
            disabled={running}
            aria-label={t("frames.seconds")}
            onChange={(e) => setSeconds(Math.max(0.2, Number(e.target.value) || 1))}
          />
        )}
        {item.lowResUrl && item.url && (
          <select value={quality} disabled={running} onChange={(e) => setQuality(e.target.value as Quality)}>
            <option value="original">{t("frames.qualityOriginal")}</option>
            <option value="preview">{t("frames.qualityPreview")}</option>
          </select>
        )}
        {running ? (
          <button type="button" className="btn btn-small" onClick={() => (cancelled.current = true)}>
            {t("frames.cancel")}
          </button>
        ) : (
          <button
            type="button"
            className="btn btn-small btn-primary"
            disabled={planned.length === 0 || !sourceUrl}
            onClick={() => void extract()}
          >
            {t("frames.extract", { count: planned.length })}
          </button>
        )}
      </div>
      {state.status === "running" && (
        <div className="frame-progress">
          <progress value={state.done} max={state.total} />
          <span className="mono small">
            {state.done} / {state.total}
          </span>
        </div>
      )}
      {state.status === "done" && (
        <p className="small">
          {t("frames.done", { count: state.count })} <span className="mono selectable">{state.index}</span>
        </p>
      )}
      {state.status === "cancelled" && <p className="small">{t("frames.cancelled", { count: state.count })}</p>}
      {error && <ErrorBanner error={error} title={t("frames.failed")} />}
      {/* Not display:none: some webviews would not decode its frames. */}
      <video ref={videoRef} className="frame-source" crossOrigin="anonymous" muted playsInline preload="metadata" />
    </section>
  );
}
