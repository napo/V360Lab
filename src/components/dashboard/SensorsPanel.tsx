import { useEffect, useState } from "react";
import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { cameraService } from "../../services/cameraService";
import type { SensorInfo } from "../../types/camera";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { ErrorBanner } from "../ErrorBanner";
import { Panel } from "../Panel";

/** Sensors paired with the camera (ANT+, Bluetooth, built-in) and the
 * folders holding media on its card. */
export function SensorsPanel() {
  const { supports } = useCamera();
  const { t } = useI18n();
  const [sensors, setSensors] = useState<SensorInfo[] | null>(null);
  const [folders, setFolders] = useState<string[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      if (supports("sensors")) setSensors(await cameraService.sensors());
      if (supports("mediaDirList")) setFolders(await cameraService.mediaDirectories());
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  return (
    <Panel
      title={t("sensors.panel")}
      actions={
        <button type="button" className="btn btn-small btn-ghost" disabled={loading} onClick={() => void load()}>
          {loading ? t("common.loading") : t("common.refresh")}
        </button>
      }
    >
      {error && <ErrorBanner error={error} />}
      {sensors && sensors.length === 0 && <p className="empty">{t("sensors.none")}</p>}
      {sensors && sensors.length > 0 && (
        <ul className="sensor-list">
          {sensors.map((sensor) => (
            <li key={`${sensor.name}-${sensor.sensorType ?? ""}`}>
              <span className={`badge-dot-inline ${sensor.found ? "on" : ""}`} aria-hidden />
              <span>{sensor.name}</span>
              {sensor.sensorType && <span className="badge">{sensor.sensorType}</span>}
              <span className="muted small">
                {sensor.found === null ? "" : sensor.found ? t("sensors.found") : t("sensors.notFound")}
              </span>
            </li>
          ))}
        </ul>
      )}
      {folders && folders.length > 0 && (
        <details className="device-commands">
          <summary>{t("sensors.folders", { count: folders.length })}</summary>
          <p className="mono small">{folders.join(", ")}</p>
        </details>
      )}
    </Panel>
  );
}
