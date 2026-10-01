import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useI18n } from "../../hooks/useI18n";
import { useVideoTelemetry, type TelemetryState } from "../../hooks/useVideoTelemetry";
import { detectAxes, levelAt } from "../../utils/level";
import { cameraMediaUrl } from "../../services/mediaUrl";
import type { MediaItem } from "../../types/camera";
import { formatDateTime, formatDuration } from "../../utils/format";
import { SphereViewer } from "../preview/SphereViewer";
import { isEquirectangular } from "../preview/sphereView";
import type { SphereSource } from "../preview/SphereRenderer";

interface MediaViewerProps {
  item: MediaItem;
  onClose: () => void;
  /** Extra content under the media (e.g. telemetry), given the playback
   * time and the video's telemetry (loaded once here). */
  children?: (time: number, seek: (seconds: number) => void, telemetry: TelemetryState) => ReactNode;
}

type Quality = "preview" | "original";

/**
 * Plays a video or shows a photo from the camera, streamed through the
 * `virb://` protocol. 360° media can be explored like the live preview.
 */
export function MediaViewer({ item, onClose, children }: MediaViewerProps) {
  const { t } = useI18n();
  const video = item.mediaType === "video";
  // The camera keeps a low-resolution copy (.GLV) of every video: it starts
  // at once and plays smoothly on phones. The original can be chosen.
  const [quality, setQuality] = useState<Quality>(video && item.lowResUrl ? "preview" : "original");
  const sourceUrl = quality === "preview" ? item.lowResUrl : item.url;
  const [element, setElement] = useState<SphereSource | null>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [failed, setFailed] = useState(false);
  const [sphereWanted, setSphereWanted] = useState(true);
  const [sphereFailed, setSphereFailed] = useState(false);
  const [resetKey, setResetKey] = useState(0);
  const [time, setTime] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [duration, setDuration] = useState(item.durationSecs ?? 0);
  const listeners = useRef(new Set<() => void>());
  const telemetry = useVideoTelemetry(video && item.hasFit ? item : null);
  const accelerometer = useMemo(() => telemetry.data?.accelerometer ?? [], [telemetry.data]);
  const axes = useMemo(() => detectAxes(accelerometer), [accelerometer]);
  const [levelOn, setLevelOn] = useState(false);

  const spherical =
    item.lensMode === "360" && !sphereFailed && size !== null && isEquirectangular(size.width, size.height);
  const sphere = spherical && sphereWanted;

  useEffect(() => {
    setSize(null);
    setFailed(false);
  }, [sourceUrl]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  // New frames for the 360° view: every decoded video frame, or once for a photo.
  useEffect(() => {
    if (!(element instanceof HTMLVideoElement)) return;
    const notify = () => listeners.current.forEach((listener) => listener());
    let handle = 0;
    let stopped = false;
    const video: HTMLVideoElement = element;
    // Not available in every webview (e.g. older WebKitGTK).
    if (typeof (video as Partial<HTMLVideoElement>).requestVideoFrameCallback === "function") {
      const onVideoFrame = () => {
        notify();
        if (!stopped) handle = video.requestVideoFrameCallback(onVideoFrame);
      };
      handle = video.requestVideoFrameCallback(onVideoFrame);
      return () => {
        stopped = true;
        video.cancelVideoFrameCallback(handle);
      };
    }
    const loop = () => {
      if (!video.paused) notify();
      if (!stopped) handle = requestAnimationFrame(loop);
    };
    handle = requestAnimationFrame(loop);
    return () => {
      stopped = true;
      cancelAnimationFrame(handle);
    };
  }, [element]);

  const subscribe = useCallback((listener: () => void) => {
    listeners.current.add(listener);
    return () => {
      listeners.current.delete(listener);
    };
  }, []);
  const sphereUnavailable = useCallback(() => setSphereFailed(true), []);

  // Horizon levelling from the accelerometer, at the frame being shown.
  const videoStartMs = telemetry.data?.videoStartMs ?? 0;
  const levelCallback = useMemo(() => {
    if (!levelOn || !axes || !(element instanceof HTMLVideoElement)) return undefined;
    return () => levelAt(accelerometer, axes, videoStartMs + element.currentTime * 1000);
  }, [levelOn, axes, element, accelerometer, videoStartMs]);

  const loaded = (width: number, height: number) => {
    setSize({ width, height });
    listeners.current.forEach((listener) => listener());
  };

  const seek = useCallback(
    (seconds: number) => {
      if (element instanceof HTMLVideoElement) element.currentTime = Math.max(0, seconds);
    },
    [element],
  );

  const togglePlay = () => {
    if (!(element instanceof HTMLVideoElement)) return;
    if (element.paused) void element.play().catch(() => {});
    else element.pause();
  };

  const src = sourceUrl ? cameraMediaUrl(sourceUrl) : null;

  return (
    <div className="media-viewer" role="dialog" aria-modal="true" aria-label={item.name}>
      <div className="media-viewer-bar">
        <div>
          <strong className="mono">{item.name}</strong>
          <span className="muted small"> · {formatDateTime(item.dateTime)}</span>
        </div>
        <span className="toolbar-spacer" />
        {video && item.lowResUrl && item.url && (
          <select
            value={quality}
            onChange={(e) => setQuality(e.target.value as Quality)}
            aria-label={t("viewer.quality")}
          >
            <option value="preview">{t("viewer.qualityPreview")}</option>
            <option value="original">{t("viewer.qualityOriginal")}</option>
          </select>
        )}
        {spherical && (
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setSphereWanted(!sphereWanted)}>
            {sphere ? t("preview.flatView") : t("preview.sphereView")}
          </button>
        )}
        {sphere && (
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setResetKey((k) => k + 1)}>
            {t("preview.lookAhead")}
          </button>
        )}
        {sphere && axes && (
          <label className="checkbox" title={t("viewer.levelHint")}>
            <input type="checkbox" checked={levelOn} onChange={(e) => setLevelOn(e.target.checked)} />
            {t("viewer.level")}
          </label>
        )}
        <button type="button" className="btn btn-small" onClick={onClose}>
          {t("viewer.close")}
        </button>
      </div>

      <div className={`media-viewer-stage ${sphere ? "sphere" : ""}`}>
        {!src ? (
          <p className="empty">{t("media.noUrl")}</p>
        ) : video ? (
          <video
            key={src}
            ref={setElement}
            src={src}
            crossOrigin="anonymous"
            playsInline
            controls={!sphere}
            hidden={sphere}
            onLoadedMetadata={(e) => {
              loaded(e.currentTarget.videoWidth, e.currentTarget.videoHeight);
              if (Number.isFinite(e.currentTarget.duration)) setDuration(e.currentTarget.duration);
            }}
            onTimeUpdate={(e) => setTime(e.currentTarget.currentTime)}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onSeeked={() => listeners.current.forEach((listener) => listener())}
            onError={() => setFailed(true)}
          />
        ) : (
          <img
            key={src}
            ref={setElement}
            src={src}
            alt={item.name}
            crossOrigin="anonymous"
            hidden={sphere}
            onLoad={(e) => loaded(e.currentTarget.naturalWidth, e.currentTarget.naturalHeight)}
            onError={() => setFailed(true)}
          />
        )}
        {sphere && (
          <SphereViewer
            source={element}
            onFrame={subscribe}
            onUnavailable={sphereUnavailable}
            resetKey={resetKey}
            levelAt={levelCallback}
          />
        )}
        {!size && !failed && src && <div className="preview-overlay">{t("common.loading")}</div>}
        {failed && (
          <div className="preview-overlay media-viewer-error">
            <span>{video ? t("viewer.videoFailed") : t("viewer.photoFailed")}</span>
            {video && quality === "preview" && item.url && (
              <button type="button" className="btn btn-small" onClick={() => setQuality("original")}>
                {t("viewer.tryOriginal")}
              </button>
            )}
          </div>
        )}
      </div>

      {video && sphere && (
        <div className="media-viewer-controls">
          <button type="button" className="btn btn-small" onClick={togglePlay}>
            {playing ? t("viewer.pause") : t("viewer.play")}
          </button>
          <input
            type="range"
            min={0}
            max={duration || 0}
            step={0.1}
            value={Math.min(time, duration || 0)}
            onChange={(e) => seek(Number(e.target.value))}
            aria-label={t("viewer.position")}
          />
          <span className="mono small">
            {formatDuration(time)} / {formatDuration(duration)}
          </span>
        </div>
      )}
      {sphere && <p className="preview-note muted small">{t("preview.sphereHint")}</p>}
      {children?.(time, seek, telemetry)}
    </div>
  );
}
