import type { ConnectionState } from "../context/CameraContext";
import { useI18n } from "../hooks/useI18n";

export function ConnectionBadge({ connection }: { connection: ConnectionState }) {
  const { t } = useI18n();
  const mock = connection.status === "connected" && connection.kind === "mock";
  return (
    <span className={`badge badge-${connection.status}`}>
      <span className="badge-dot" />
      {t(`connection.${connection.status}`)}
      {mock && t("connection.mockSuffix")}
    </span>
  );
}
