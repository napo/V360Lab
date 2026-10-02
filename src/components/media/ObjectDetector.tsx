import { useState } from "react";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import { cameraService } from "../../services/cameraService";
import type { DetectionBenchmark, DetectionProgress, DetectionReport, MediaItem } from "../../types/camera";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { formatDurationRough, remainingMs } from "../../utils/eta";

/** Above this, detection on this device takes too long to be practical. */
const SLOW_FRAME_MS = 30_000;

// Measured once per session: the result does not change.
let cachedBenchmark: { model: string; result: DetectionBenchmark } | null = null;
import { ErrorBanner } from "../ErrorBanner";
import { ModelManager } from "../ModelManager";

/**
 * Object detection (YOLO, ONNX) on the frames extracted from a video:
 * writes detections.geojson with the position of each frame and the
 * direction of each object.
 */
export function ObjectDetector({ item }: { item: MediaItem }) {
  const { t } = useI18n();
  const { view } = useSettings();
  const [showModels, setShowModels] = useState(false);
  const model = view?.settings.detectionModel ?? null;
  const [confidence, setConfidence] = useState(0.4);
  const [progress, setProgress] = useState<DetectionProgress | null>(null);
  const [report, setReport] = useState<DetectionReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const running = progress !== null;
  const [benchmark, setBenchmark] = useState<DetectionBenchmark | null>(
    cachedBenchmark && cachedBenchmark.model === model ? cachedBenchmark.result : null,
  );
  const [measuring, setMeasuring] = useState(false);
  const [startedAt, setStartedAt] = useState(0);

  const measure = async () => {
    setError(null);
    setMeasuring(true);
    try {
      const result = await cameraService.benchmarkDetection();
      if (model) cachedBenchmark = { model, result };
      setBenchmark(result);
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setMeasuring(false);
    }
  };

  // Before the first frame: the measured speed times the number of frames;
  // afterwards: the time actually spent per frame.
  const remaining =
    progress && progress.total > 0
      ? (remainingMs(Date.now() - startedAt, progress.done, progress.total) ??
        (benchmark ? benchmark.frameMs * progress.total : null))
      : null;

  const run = async () => {
    setError(null);
    setReport(null);
    setProgress({ done: 0, total: 0, detections: 0 });
    setStartedAt(Date.now());
    try {
      setReport(await cameraService.detectObjects(item, confidence, setProgress));
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setProgress(null);
    }
  };

  const modelName = model ? model.split(/[\\/]/).pop() : null;

  return (
    <section className="object-detector" aria-label={t("detect.title")}>
      <h3>{t("detect.title")}</h3>
      <p className="muted small">{t("detect.intro")}</p>
      {model ? (
        <div className="frame-options">
          <span className="small">
            {t("detect.model")}: <span className="mono">{modelName}</span>
          </span>
          <button
            type="button"
            className="btn btn-small btn-ghost"
            disabled={running}
            onClick={() => setShowModels(!showModels)}
          >
            {t("detect.changeModel")}
          </button>
        </div>
      ) : (
        <p className="small">{t("detect.needModel")}</p>
      )}
      {(!model || showModels) && <ModelManager />}
      {model && (
        <div className="frame-options">
          <button
            type="button"
            className="btn btn-small btn-ghost"
            disabled={running || measuring}
            onClick={() => void measure()}
          >
            {measuring ? t("detect.measuring") : t("detect.measure")}
          </button>
          {benchmark && (
            <span className="small">
              {t("detect.speed", {
                frame: formatDurationRough(benchmark.frameMs),
                hundred: formatDurationRough(benchmark.frameMs * 100),
              })}
            </span>
          )}
        </div>
      )}
      {benchmark && benchmark.frameMs > SLOW_FRAME_MS && <p className="small warning-text">{t("detect.slow")}</p>}
      <div className="frame-options">
        <label className="small">
          {t("detect.confidence", { value: Math.round(confidence * 100) })}
          <input
            type="range"
            min={0.1}
            max={0.9}
            step={0.05}
            value={confidence}
            disabled={running}
            onChange={(e) => setConfidence(Number(e.target.value))}
          />
        </label>
        {running ? (
          <button type="button" className="btn btn-small" onClick={() => void cameraService.cancelDetection()}>
            {t("frames.cancel")}
          </button>
        ) : (
          <button type="button" className="btn btn-small btn-primary" disabled={!model} onClick={() => void run()}>
            {t("detect.run")}
          </button>
        )}
      </div>
      {progress && (
        <div className="frame-progress">
          <progress value={progress.done} max={progress.total || 1} />
          <span className="mono small">
            {progress.done} / {progress.total || "…"} · {t("detect.found", { count: progress.detections })}
            {remaining !== null && ` · ${t("detect.remaining", { time: formatDurationRough(remaining) })}`}
          </span>
        </div>
      )}
      {report && (
        <div className="small">
          <p>
            {t(report.cancelled ? "detect.doneCancelled" : "detect.done", {
              count: report.detections,
              frames: report.frames,
            })}{" "}
            <span className="mono selectable">{report.path}</span>
          </p>
          {report.byClass.length > 0 && (
            <ul className="detect-classes">
              {report.byClass.map(([name, count]) => (
                <li key={name}>
                  <span className="badge">{name}</span> {count}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      {error && <ErrorBanner error={error} title={t("detect.failed")} />}
    </section>
  );
}
