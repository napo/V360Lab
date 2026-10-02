import { useState } from "react";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import { cameraService } from "../../services/cameraService";
import { settingsService } from "../../services/settingsService";
import type { DetectionProgress, DetectionReport, MediaItem } from "../../types/camera";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { ErrorBanner } from "../ErrorBanner";

/**
 * Object detection (YOLO, ONNX) on the frames extracted from a video:
 * writes detections.geojson with the position of each frame and the
 * direction of each object.
 */
export function ObjectDetector({ item }: { item: MediaItem }) {
  const { t } = useI18n();
  const { view, save } = useSettings();
  const model = view?.settings.detectionModel ?? null;
  const [confidence, setConfidence] = useState(0.4);
  const [progress, setProgress] = useState<DetectionProgress | null>(null);
  const [report, setReport] = useState<DetectionReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const running = progress !== null;

  const chooseModel = async () => {
    try {
      const path = await settingsService.pickModel(t("detect.chooseModel"));
      if (path && view) await save({ ...view.settings, detectionModel: path });
    } catch (e) {
      setError(toAppError(e));
    }
  };

  const run = async () => {
    setError(null);
    setReport(null);
    setProgress({ done: 0, total: 0, detections: 0 });
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
      <div className="frame-options">
        <span className="small">
          {t("detect.model")}: <span className="mono">{modelName ?? t("detect.noModel")}</span>
        </span>
        <button type="button" className="btn btn-small btn-ghost" disabled={running} onClick={() => void chooseModel()}>
          {model ? t("detect.changeModel") : t("detect.chooseModel")}
        </button>
      </div>
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
