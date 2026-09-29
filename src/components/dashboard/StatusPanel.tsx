import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import {
  formatBytes,
  formatCoordinate,
  formatDuration,
  formatPercent,
  formatShootingMode,
  storageUsedFraction,
} from "../../utils/format";
import { ErrorBanner } from "../ErrorBanner";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { Meter } from "../Meter";
import { Panel } from "../Panel";

export function StatusPanel() {
  const { status, statusError, statusUpdatedAt, shootingMode, refreshing } = useCamera();
  const { debugMode } = useSettings();
  const { t, language } = useI18n();

  const battery = status?.batteryLevel ?? null;
  const used = storageUsedFraction(status?.storageTotalBytes, status?.storageAvailableBytes);
  const recording = status?.recordingState ?? "unknown";

  return (
    <Panel
      title={t("status.panel")}
      actions={
        <span className="muted small">
          {refreshing
            ? t("status.refreshing")
            : statusUpdatedAt &&
              t("status.updatedAt", { time: statusUpdatedAt.toLocaleTimeString(language) })}
        </span>
      }
    >
      {statusError && <ErrorBanner error={statusError} title={t("status.unavailable")} />}
      <KeyValueList
        items={[
          {
            label: t("status.recording"),
            value: (
              <span className={`state state-${recording}`}>
                {t(`recording.${recording}`)}
                {recording === "recording" && ` · ${formatDuration(status?.recordingTimeSecs)}`}
              </span>
            ),
          },
          { label: t("status.mode"), value: status?.mode ?? formatShootingMode(shootingMode) },
          {
            label: t("status.battery"),
            value:
              battery == null ? null : (
                <div className="meter-row">
                  <Meter
                    fraction={battery / 100}
                    tone={battery < 15 ? "danger" : battery < 30 ? "warning" : "default"}
                  />
                  <span className="mono">{formatPercent(battery)}</span>
                </div>
              ),
          },
          {
            label: t("status.storage"),
            value:
              used == null ? null : (
                <div className="meter-row">
                  <Meter fraction={used} tone={used > 0.95 ? "danger" : used > 0.85 ? "warning" : "default"} />
                  <span className="mono">
                    {t("status.storageFree", {
                      free: formatBytes(status?.storageAvailableBytes),
                      total: formatBytes(status?.storageTotalBytes),
                    })}
                  </span>
                </div>
              ),
          },
          {
            label: t("status.remainingTime"),
            value:
              status?.recordingTimeRemainingSecs != null
                ? formatDuration(status.recordingTimeRemainingSecs)
                : null,
            mono: true,
          },
          {
            label: t("status.gpsPosition"),
            value:
              status?.gpsLatitude != null && status?.gpsLongitude != null
                ? `${formatCoordinate(status.gpsLatitude)}, ${formatCoordinate(status.gpsLongitude)}`
                : null,
            mono: true,
          },
        ]}
      />
      {debugMode && status && <JsonViewer value={status.raw} title={t("status.raw")} />}
    </Panel>
  );
}
