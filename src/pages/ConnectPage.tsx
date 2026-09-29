import { BrandHero } from "../components/BrandHero";
import { KeyValueList } from "../components/KeyValueList";
import type { Navigate } from "../components/navigation";
import { Panel } from "../components/Panel";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { formatBytes, formatPercent } from "../utils/format";
import { ConnectionPage } from "./ConnectionPage";

/** "Connect" section: search when disconnected, a summary when connected. */
export function ConnectPage({ navigate }: { navigate: Navigate }) {
  const { connection, deviceInfo, status, disconnect } = useCamera();
  const { t } = useI18n();

  if (connection.status !== "connected") return <ConnectionPage />;

  return (
    <div className="page page-narrow">
      <BrandHero />
      <Panel title={t("connect.connectedTitle")}>
        <KeyValueList
          items={[
            { label: t("device.model"), value: deviceInfo?.model },
            { label: t("device.firmware"), value: deviceInfo?.firmware, mono: true },
            { label: t("device.address"), value: connection.address, mono: true },
            { label: t("status.battery"), value: status?.batteryLevel != null ? formatPercent(status.batteryLevel) : null },
            {
              label: t("status.storage"),
              value:
                status?.storageAvailableBytes != null
                  ? t("status.storageFree", {
                      free: formatBytes(status.storageAvailableBytes),
                      total: formatBytes(status.storageTotalBytes),
                    })
                  : null,
            },
          ]}
        />
        <div className="form-actions connected-actions">
          <button type="button" className="btn btn-primary" onClick={() => navigate("capture")}>
            {t("nav.capture")}
          </button>
          <button type="button" className="btn" onClick={() => navigate("media")}>
            {t("nav.media")}
          </button>
          <button type="button" className="btn btn-ghost" onClick={() => void disconnect()}>
            {t("sidebar.disconnect")}
          </button>
        </div>
      </Panel>
    </div>
  );
}
