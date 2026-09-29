import type { ConnectionState } from "../context/CameraContext";

const LABELS: Record<ConnectionState["status"], string> = {
  disconnected: "Disconnected",
  connecting: "Connecting…",
  connected: "Connected",
  error: "Error",
};

export function ConnectionBadge({ connection }: { connection: ConnectionState }) {
  const mock = connection.status === "connected" && connection.kind === "mock";
  return (
    <span className={`badge badge-${connection.status}`}>
      <span className="badge-dot" />
      {LABELS[connection.status]}
      {mock && " (mock)"}
    </span>
  );
}
