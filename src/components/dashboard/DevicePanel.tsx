import { useCamera } from "../../hooks/useCamera";
import { useSettings } from "../../hooks/useSettings";
import { ConnectionBadge } from "../ConnectionBadge";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { Panel } from "../Panel";

export function DevicePanel() {
  const { connection, deviceInfo } = useCamera();
  const { debugMode } = useSettings();
  const address = connection.status === "connected" ? connection.address : null;

  return (
    <Panel title="Camera">
      <KeyValueList
        items={[
          { label: "Model", value: deviceInfo?.model },
          { label: "Firmware", value: deviceInfo?.firmware, mono: true },
          { label: "Device ID", value: deviceInfo?.deviceId, mono: true },
          { label: "Part number", value: deviceInfo?.partNumber, mono: true },
          { label: "Address", value: address, mono: true },
          { label: "Connection", value: <ConnectionBadge connection={connection} /> },
        ]}
      />
      {debugMode && deviceInfo && <JsonViewer value={deviceInfo.raw} title="Raw device info" />}
    </Panel>
  );
}
