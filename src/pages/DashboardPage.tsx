import { ActionsPanel } from "../components/dashboard/ActionsPanel";
import { DevicePanel } from "../components/dashboard/DevicePanel";
import { StatusPanel } from "../components/dashboard/StatusPanel";

export function DashboardPage() {
  return (
    <div className="page">
      <h1>Dashboard</h1>
      <div className="grid grid-dashboard">
        <DevicePanel />
        <StatusPanel />
        <ActionsPanel />
      </div>
    </div>
  );
}
