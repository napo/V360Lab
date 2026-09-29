import { useState } from "react";
import { DevicePanel } from "../components/dashboard/DevicePanel";
import { StatusPanel } from "../components/dashboard/StatusPanel";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import type { TranslationKey } from "../i18n/types";
import { AboutPage } from "./AboutPage";
import { FeaturesPage } from "./FeaturesPage";
import { SettingsPage } from "./SettingsPage";

type Tab = "camera" | "device" | "app" | "about";

const TABS: Array<{ id: Tab; label: TranslationKey; needsCamera: boolean }> = [
  { id: "camera", label: "advanced.cameraSettings", needsCamera: true },
  { id: "device", label: "advanced.device", needsCamera: true },
  { id: "app", label: "advanced.app", needsCamera: false },
  { id: "about", label: "advanced.about", needsCamera: false },
];

/** Everything that is not needed to capture: camera settings, device
 * details, app settings and information. */
export function AdvancedPage() {
  const { connection } = useCamera();
  const { t } = useI18n();
  const connected = connection.status === "connected";
  const [tab, setTab] = useState<Tab>(connected ? "camera" : "app");
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
            onClick={() => setTab(entry.id)}
          >
            {t(entry.label)}
          </button>
        ))}
      </div>
      <div className="advanced-content">
        {current.needsCamera && !connected ? (
          <p className="empty">{t("advanced.needsCamera")}</p>
        ) : tab === "camera" ? (
          <FeaturesPage />
        ) : tab === "device" ? (
          <div className="grid grid-dashboard">
            <StatusPanel />
            <DevicePanel />
          </div>
        ) : tab === "app" ? (
          <SettingsPage />
        ) : (
          <AboutPage />
        )}
      </div>
    </div>
  );
}
