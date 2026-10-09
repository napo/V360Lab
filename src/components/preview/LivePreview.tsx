import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "../../hooks/useI18n";
import { usePageVisible } from "../../hooks/usePageVisible";
import { previewService } from "../../services/previewService";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { ErrorBanner } from "../ErrorBanner";
import { Spinner } from "../Spinner";
import { PreviewPlayer, webCodecsAvailable } from "./PreviewPlayer";
import { SphereViewer } from "./SphereViewer";
import { isEquirectangular } from "./sphereView";

type State = "off" | "starting" | "playing" | "reconnecting" | "error";

const STORAGE_KEY = "v360lab.preview.enabled";
const SPHERE_KEY = "v360lab.preview.sphere";
/** Automatic reconnections after the stream stops, before giving up. */
const MAX_RETRIES = 3;
const RETRY_DELAY_MS = 1500;
/**
 * The VIRB stops the preview stream when the lens or mode changes, and
 * needs a moment before streaming again reliably (measured on 4.20).
 */
const RESTART_AFTER_CHANGE_MS = 2000;

function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) !== "false";
  } catch {
    return true;
  }
}

function writeFlag(key: string, value: boolean) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    // Preference only.
  }
}

/**
 * What the camera sees right now, with the current lens and mode. Starts
 * automatically (unless the user turned it off) and stops when unmounted.
 * To save the camera's battery it also stops while the app is in the
 * background, and during a recording unless the user asks for it.
 */
interface LivePreviewProps {
  /** Changes whenever a setting that restarts the camera's stream changes. */
  restartKey?: string;
  /** The camera films 360°: the preview is then an equirectangular image. */
  spherical?: boolean;
  /** The camera is recording. */
  recording?: boolean;
}

export function LivePreview({ restartKey, spherical = false, recording = false }: LivePreviewProps) {
  const { t } = useI18n();
  const supported = webCodecsAvailable();
  const visible = usePageVisible();
  const [enabled, setEnabled] = useState(() => readFlag(STORAGE_KEY));
  // Shown during the current recording on request; asked again next time.
  const [showWhileRecording, setShowWhileRecording] = useState(false);
  if (!recording && showWhileRecording) setShowWhileRecording(false);
  const pausedForRecording = recording && !showWhileRecording;
  const active = supported && enabled && visible && !pausedForRecording;
  const [sphereWanted, setSphereWanted] = useState(() => readFlag(SPHERE_KEY));
  const [sphereFailed, setSphereFailed] = useState(false);
  const [resetKey, setResetKey] = useState(0);
  const [decodeCanvas, setDecodeCanvas] = useState<HTMLCanvasElement | null>(null);
  const frameListeners = useRef(new Set<() => void>());
  const [state, setState] = useState<State>("off");
  const [error, setError] = useState<AppError | null>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  // Each start gets a new generation; messages of older ones are ignored.
  const generation = useRef(0);
  const player = useRef<PreviewPlayer | null>(null);
  const retries = useRef(0);
  const retryTimer = useRef<number | undefined>(undefined);

  const stop = useCallback(() => {
    generation.current++;
    window.clearTimeout(retryTimer.current);
    player.current?.close();
    player.current = null;
    void previewService.stop().catch(() => {});
  }, []);

  const start = useCallback(
    async (reconnecting = false) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const current = ++generation.current;
      window.clearTimeout(retryTimer.current);
      player.current?.close();
      setError(null);
      setState(reconnecting ? "reconnecting" : "starting");

      const instance = new PreviewPlayer(canvas, {
        onFirstFrame: (width, height) => {
          if (generation.current !== current) return;
          retries.current = 0;
          setSize({ width, height });
          setState("playing");
        },
        onFrame: () => frameListeners.current.forEach((listener) => listener()),
        onDecodeError: (message) => console.warn("Live preview decode error:", message),
      });
      player.current = instance;

      try {
        await previewService.start((message) => {
          if (generation.current !== current) return;
          if (message.type !== "ended") {
            instance.handle(message);
            return;
          }
          instance.close();
          // We never stop the current generation ourselves, so a normal end
          // is unexpected too (e.g. a stop that overtook this start).
          const reason = message.error === null ? "stalled" : message.error.params?.reason;
          if (reason === "stalled" && retries.current < MAX_RETRIES) {
            retries.current++;
            setState("reconnecting");
            retryTimer.current = window.setTimeout(() => void start(true), RETRY_DELAY_MS);
          } else {
            setError(
              message.error ?? {
                kind: "preview",
                message: "The live preview stopped",
                detail: null,
                params: { reason: "stalled" },
              },
            );
            setState("error");
          }
        });
      } catch (e) {
        if (generation.current !== current) return;
        instance.close();
        setError(toAppError(e));
        setState("error");
      }
    },
    [],
  );

  useEffect(() => {
    if (!active) {
      setState("off");
      return;
    }
    retries.current = 0;
    void start();
    return stop;
  }, [active, start, stop]);

  // Restart right after a lens/mode change instead of waiting for the
  // stalled-stream detection.
  const lastKey = useRef(restartKey);
  useEffect(() => {
    if (lastKey.current === restartKey) return;
    lastKey.current = restartKey;
    if (!active) return;
    generation.current++;
    window.clearTimeout(retryTimer.current);
    player.current?.close();
    player.current = null;
    setError(null);
    setState("reconnecting");
    retries.current = 0;
    retryTimer.current = window.setTimeout(() => void start(true), RESTART_AFTER_CHANGE_MS);
  }, [restartKey, active, start]);

  const subscribeFrames = useCallback((listener: () => void) => {
    frameListeners.current.add(listener);
    return () => {
      frameListeners.current.delete(listener);
    };
  }, []);
  const sphereUnavailable = useCallback(() => setSphereFailed(true), []);
  const decodeCanvasRef = useCallback((canvas: HTMLCanvasElement | null) => {
    canvasRef.current = canvas;
    setDecodeCanvas(canvas);
  }, []);

  // The camera sends a 2:1 equirectangular image in 360° mode; anything else
  // (single lens, RAW) is shown as it is.
  const sphereAvailable =
    spherical && !sphereFailed && size !== null && isEquirectangular(size.width, size.height);
  const sphere = sphereAvailable && sphereWanted;

  const toggleSphere = () => {
    const next = !sphereWanted;
    writeFlag(SPHERE_KEY, next);
    setSphereWanted(next);
  };

  const toggle = () => {
    const next = !enabled;
    writeFlag(STORAGE_KEY, next);
    setEnabled(next);
    if (!next) setState("off");
  };

  const fullscreen = () => {
    const element = frameRef.current;
    if (!element) return;
    if (document.fullscreenElement) void document.exitFullscreen();
    else void element.requestFullscreen?.().catch(() => {});
  };

  if (!supported) {
    return <p className="preview-note muted small">{t("preview.unsupportedBrowser")}</p>;
  }

  return (
    <section className="live-preview" aria-label={t("preview.title")}>
      {enabled && (
        <div
          ref={frameRef}
          className={`preview-frame ${state} ${sphere ? "sphere" : ""}`}
          style={size && !sphere ? { aspectRatio: `${size.width} / ${size.height}` } : undefined}
          onDoubleClick={sphere ? undefined : fullscreen}
        >
          <canvas ref={decodeCanvasRef} hidden={sphere} />
          {sphere && (
            <SphereViewer
              source={decodeCanvas}
              onFrame={subscribeFrames}
              onUnavailable={sphereUnavailable}
              resetKey={resetKey}
            />
          )}
          {state === "playing" && <span className="preview-live">{t("preview.live")}</span>}
          {(state === "starting" || state === "reconnecting") && (
            <div className="preview-overlay">
              <Spinner size={28} />
              <span>{state === "starting" ? t("preview.starting") : t("preview.reconnecting")}</span>
            </div>
          )}
          {enabled && pausedForRecording && (
            <div className="preview-overlay">
              <span className="preview-paused">{t("preview.pausedRecording")}</span>
              <button type="button" className="btn btn-small" onClick={() => setShowWhileRecording(true)}>
                {t("preview.showAnyway")}
              </button>
            </div>
          )}
          {state === "error" && (
            <div className="preview-overlay">
              <button type="button" className="btn btn-small" onClick={() => void start()}>
                {t("preview.retry")}
              </button>
            </div>
          )}
        </div>
      )}
      <div className="preview-actions">
        <button type="button" className="btn btn-small btn-ghost" onClick={toggle}>
          {enabled ? t("preview.hide") : t("preview.show")}
        </button>
        {enabled && state === "playing" && sphereAvailable && (
          <button type="button" className="btn btn-small btn-ghost" onClick={toggleSphere}>
            {sphere ? t("preview.flatView") : t("preview.sphereView")}
          </button>
        )}
        {enabled && state === "playing" && sphere && (
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setResetKey((k) => k + 1)}>
            {t("preview.lookAhead")}
          </button>
        )}
        {enabled && state === "playing" && (
          <button type="button" className="btn btn-small btn-ghost" onClick={fullscreen}>
            {t("preview.fullscreen")}
          </button>
        )}
      </div>
      {enabled && state === "playing" && sphere && (
        <p className="preview-note muted small">{t("preview.sphereHint")}</p>
      )}
      {error && <ErrorBanner error={error} title={t("preview.failed")} />}
    </section>
  );
}
