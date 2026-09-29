import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { Panel } from "../components/Panel";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";

/** Shown instead of camera pages while no camera is connected. */
export function ConnectionPage() {
  const { connection, connect } = useCamera();
  const { view } = useSettings();
  const { t, tx } = useI18n();
  const [address, setAddress] = useState("");
  const [mock, setMock] = useState(false);
  const [initialized, setInitialized] = useState(false);

  // Prefill from the last successful address once settings are loaded.
  useEffect(() => {
    if (!view || initialized) return;
    setAddress(view.settings.lastCameraAddress ?? view.defaultCameraAddress);
    setMock(view.settings.mockMode);
    setInitialized(true);
  }, [view, initialized]);

  const connecting = connection.status === "connecting";
  const defaultAddress = view?.defaultCameraAddress ?? "192.168.0.1";

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!connecting) void connect(address, mock);
  };

  return (
    <div className="page page-narrow">
      <div className="brand-hero">
        <img className="logo-light" src="/brand/logo-light-bg.png" alt={t("app.name")} />
        <img className="logo-dark" src="/brand/logo-dark-bg.png" alt={t("app.name")} />
      </div>
      <h1>{t("connect.title")}</h1>
      <Panel title={t("connect.addressPanel")}>
        <form className="form" onSubmit={submit}>
          <label className="field">
            <span>{t("connect.addressLabel")}</span>
            <input
              className="mono"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder={defaultAddress}
              disabled={connecting || mock}
              spellCheck={false}
              autoFocus
            />
          </label>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={mock}
              onChange={(e) => setMock(e.target.checked)}
              disabled={connecting}
            />
            {t("connect.useMock")}
          </label>
          <div className="form-actions">
            <button
              type="submit"
              className="btn btn-primary"
              disabled={connecting || (!mock && !address.trim())}
            >
              {connecting ? t("connect.connecting") : t("connect.connect")}
            </button>
            <span className="muted">
              {connecting &&
                t("connect.contacting", {
                  target: connection.mock ? t("connect.mockCamera") : connection.address,
                })}
              {connection.status === "disconnected" && t("connect.notConnected")}
            </span>
          </div>
        </form>
        {connection.status === "error" && (
          <ErrorBanner error={connection.error} title={t("connect.failed")} />
        )}
      </Panel>
      <Panel title={t("connect.requirements")}>
        <ul className="hint-list">
          <li>{t("connect.requirementWifi")}</li>
          <li>{tx("connect.requirementAddress", { address: <code>{defaultAddress}</code> })}</li>
          <li>{t("connect.requirementLocal")}</li>
        </ul>
      </Panel>
    </div>
  );
}
