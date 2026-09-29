import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import { ConnectionBadge } from "../ConnectionBadge";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { Panel } from "../Panel";

export function DevicePanel() {
  const { connection, deviceInfo } = useCamera();
  const { debugMode } = useSettings();
  const { t } = useI18n();
  const address = connection.status === "connected" ? connection.address : null;

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
      {debugMode && deviceInfo && <JsonViewer value={deviceInfo.raw} title={t("device.raw")} />}
    </Panel>
  );
}
