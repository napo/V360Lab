import { useCamera } from "../../hooks/useCamera";
import { useSettings } from "../../hooks/useSettings";
import type { RecordingState } from "../../types/camera";
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

const RECORDING_LABELS: Record<RecordingState, string> = {
  idle: "Idle",
  recording: "Recording",
  unknown: "Unknown",
};

export function StatusPanel() {
  const { status, statusError, statusUpdatedAt, shootingMode, refreshing } = useCamera();
  const { debugMode } = useSettings();

  const battery = status?.batteryLevel ?? null;
  const used = storageUsedFraction(status?.storageTotalBytes, status?.storageAvailableBytes);
  const recording = status?.recordingState ?? "unknown";

  return (
    <Panel
      title="Status"
      actions={
        <span className="muted small">
          {refreshing ? "Refreshing…" : statusUpdatedAt && `Updated ${statusUpdatedAt.toLocaleTimeString()}`}
        </span>
      }
    >
      {statusError && <ErrorBanner error={statusError} title="Status unavailable" />}
      <KeyValueList
        items={[
          {
            label: "Recording",
            value: (
              <span className={`state state-${recording}`}>
                {RECORDING_LABELS[recording]}
                {recording === "recording" && ` · ${formatDuration(status?.recordingTimeSecs)}`}
              </span>
            ),
          },
          { label: "Mode", value: status?.mode ?? formatShootingMode(shootingMode) },
          {
            label: "Battery",
            value:
              battery == null ? null : (
                <div className="meter-row">
                  <Meter fraction={battery / 100} tone={battery < 15 ? "danger" : battery < 30 ? "warning" : "default"} />
                  <span className="mono">{formatPercent(battery)}</span>
                  {status?.batteryChargingState && <span className="muted">{status.batteryChargingState}</span>}
                </div>
              ),
          },
          {
            label: "Storage",
            value:
              used == null ? null : (
                <div className="meter-row">
                  <Meter fraction={used} tone={used > 0.95 ? "danger" : used > 0.85 ? "warning" : "default"} />
                  <span className="mono">
                    {formatBytes(status?.storageAvailableBytes)} free of {formatBytes(status?.storageTotalBytes)}
                  </span>
                </div>
              ),
          },
          {
            label: "Remaining time",
            value:
              status?.recordingTimeRemainingSecs != null
                ? formatDuration(status.recordingTimeRemainingSecs)
                : null,
            mono: true,
          },
          {
            label: "GPS position",
            value:
              status?.gpsLatitude != null && status?.gpsLongitude != null
                ? `${formatCoordinate(status.gpsLatitude)}, ${formatCoordinate(status.gpsLongitude)}`
                : null,
            mono: true,
          },
        ]}
      />
      {debugMode && status && <JsonViewer value={status.raw} title="Raw status" />}
    </Panel>
  );
}
