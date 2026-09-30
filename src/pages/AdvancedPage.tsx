import { useState } from "react";
import { DevicePanel } from "../components/dashboard/DevicePanel";
import { StatusPanel } from "../components/dashboard/StatusPanel";
import type { Navigate } from "../components/navigation";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import type { TranslationKey } from "../i18n/types";
import type { WifiSwitch } from "../types/camera";
import { AboutPage } from "./AboutPage";
import { FeaturesPage } from "./FeaturesPage";
import { SettingsPage } from "./SettingsPage";
import { WifiPage } from "./WifiPage";

type Tab = "camera" | "device" | "wifi" | "app" | "about";

const TABS: Array<{ id: Tab; label: TranslationKey; needsCamera: boolean }> = [
  { id: "camera", label: "advanced.cameraSettings", needsCamera: true },
  { id: "device", label: "advanced.device", needsCamera: true },
  { id: "wifi", label: "advanced.wifi", needsCamera: true },
  { id: "app", label: "advanced.app", needsCamera: false },
  { id: "about", label: "advanced.about", needsCamera: false },
];

/** Everything that is not needed to capture: camera settings, device
 * details, app settings and information. */
export function AdvancedPage({ navigate }: { navigate: Navigate }) {
  const { connection } = useCamera();
  const { t } = useI18n();
  const connected = connection.status === "connected";
  const [tab, setTab] = useState<Tab>(connected ? "camera" : "app");
  const [switched, setSwitched] = useState<{ ssid: string; result: WifiSwitch } | null>(null);
  const current = TABS.find((entry) => entry.id === tab)!;

  return (
    <div className="page advanced-page">
      <h1>{t("advanced.title")}</h1>
      <div className="tabs" role="tablist">
        {TABS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={entry.id === tab}
            className={entry.id === tab ? "active" : undefined}
            onClick={() => {
              setTab(entry.id);
              setSwitched(null);
            }}
          >
            {t(entry.label)}
          </button>
        ))}
      </div>
      <div className="advanced-content">
        {switched ? (
          <div className="page page-narrow">
            <h2>{t("wifi.switchedTitle", { ssid: switched.ssid })}</h2>
            {!switched.result.confirmed && <p className="muted">{t("wifi.switchedUnconfirmed")}</p>}
            <ol className="wifi-steps">
              <li>{t("wifi.switchedStep1", { ssid: switched.ssid })}</li>
              <li>{t("wifi.switchedStep2", { ssid: switched.ssid })}</li>
              <li>{t("wifi.switchedStep3")}</li>
            </ol>
            <p className="muted small">{t("wifi.switchedFallback")}</p>
            <button type="button" className="btn btn-primary" onClick={() => navigate("connect")}>
              {t("capture.goToConnect")}
            </button>
          </div>
        ) : current.needsCamera && !connected ? (
          <p className="empty">{t("advanced.needsCamera")}</p>
        ) : tab === "camera" ? (
          <FeaturesPage />
        ) : tab === "device" ? (
          <div className="grid grid-dashboard">
            <StatusPanel />
            <DevicePanel />
          </div>
        ) : tab === "wifi" ? (
          <WifiPage onSwitched={(ssid, result) => setSwitched({ ssid, result })} />
        ) : tab === "app" ? (
          <SettingsPage />
        ) : (
          <AboutPage />
        )}
      </div>
    </div>
  );
}
