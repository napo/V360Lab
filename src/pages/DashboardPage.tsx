import { ActionsPanel } from "../components/dashboard/ActionsPanel";
import { DevicePanel } from "../components/dashboard/DevicePanel";
import { StatusPanel } from "../components/dashboard/StatusPanel";
import { useI18n } from "../hooks/useI18n";

export function DashboardPage() {
  const { t } = useI18n();
  return (
    <div className="page">
      <h1>{t("dashboard.title")}</h1>
      <div className="grid grid-dashboard">
        <DevicePanel />
        <StatusPanel />
        <ActionsPanel />
      </div>
    </div>
  );
}
