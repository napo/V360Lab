import { useI18n } from "../hooks/useI18n";
import type { Navigate } from "./navigation";

/** Placeholder for sections that need a camera. */
export function NotConnected({ navigate }: { navigate: Navigate }) {
  const { t } = useI18n();
  return (
    <div className="page page-narrow center">
      <p className="muted">{t("capture.notConnected")}</p>
      <button type="button" className="btn btn-primary btn-hero" onClick={() => navigate("connect")}>
        {t("capture.goToConnect")}
      </button>
    </div>
  );
}
