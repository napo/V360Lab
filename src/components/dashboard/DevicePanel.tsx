import { useEffect, useRef, useState } from "react";
import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import { ConnectionBadge } from "../ConnectionBadge";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { Panel } from "../Panel";
import { ErrorBanner } from "../ErrorBanner";
import { cameraService } from "../../services/cameraService";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";

export function DevicePanel() {
  const { connection, deviceInfo, supportedCommands, supports } = useCamera();
  const { debugMode } = useSettings();
  const { t } = useI18n();
  const address = connection.status === "connected" ? connection.address : null;
  const [locating, setLocating] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  // Leaving the panel stops the signal: nothing else could stop it later.
  const locatingRef = useRef(false);
  locatingRef.current = locating;
  useEffect(
    () => () => {
      if (locatingRef.current) void cameraService.locate(false).catch(() => {});
    },
    [],
  );

  // `locate` makes the camera beep and blink until `found`.
  const toggleLocate = async () => {
    setBusy(true);
    setError(null);
    try {
      await cameraService.locate(!locating);
      setLocating(!locating);
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title={t("device.panel")}>
      <KeyValueList
        items={[
          { label: t("device.model"), value: deviceInfo?.model },
          { label: t("device.firmware"), value: deviceInfo?.firmware, mono: true },
          { label: t("device.deviceId"), value: deviceInfo?.deviceId, mono: true },
          { label: t("device.partNumber"), value: deviceInfo?.partNumber, mono: true },
          { label: t("device.address"), value: address, mono: true },
          { label: t("device.connection"), value: <ConnectionBadge connection={connection} /> },
        ]}
      />
      {address && supports("locate") && (
        <div className="form-actions">
          <button
            type="button"
            className={`btn ${locating ? "btn-primary" : ""}`}
            disabled={busy}
            onClick={() => void toggleLocate()}
          >
            {locating ? t("device.locateStop") : t("device.locateStart")}
          </button>
          <span className="muted small">{t("device.locateHint")}</span>
        </div>
      )}
      {error && <ErrorBanner error={error} />}
      {supportedCommands && supportedCommands.length > 0 && (
        <details className="device-commands">
          <summary>{t("device.supportedCommands", { count: supportedCommands.length })}</summary>
          <p className="mono small">{supportedCommands.join(", ")}</p>
        </details>
      )}
      {debugMode && deviceInfo && <JsonViewer value={deviceInfo.raw} title={t("device.raw")} />}
    </Panel>
  );
}
