import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { Panel } from "../components/Panel";
import { useCamera } from "../hooks/useCamera";
import { useSettings } from "../hooks/useSettings";

/** Shown instead of camera pages while no camera is connected. */
export function ConnectionPage() {
  const { connection, connect } = useCamera();
  const { view } = useSettings();
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

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!connecting) void connect(address, mock);
  };

  return (
    <div className="page page-narrow">
      <h1>Connect to camera</h1>
      <Panel title="Camera address">
        <form className="form" onSubmit={submit}>
          <label className="field">
            <span>IP address or host name</span>
            <input
              className="mono"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder={view?.defaultCameraAddress ?? "192.168.0.1"}
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
            Use simulated camera (mock mode, no network access)
          </label>
          <div className="form-actions">
            <button
              type="submit"
              className="btn btn-primary"
              disabled={connecting || (!mock && !address.trim())}
            >
              {connecting ? "Connecting…" : "Connect"}
            </button>
            <span className="muted">
              {connecting && `Contacting ${connection.mock ? "mock camera" : connection.address}…`}
              {connection.status === "disconnected" && "Not connected"}
            </span>
          </div>
        </form>
        {connection.status === "error" && (
          <ErrorBanner error={connection.error} title="Connection failed" />
        )}
      </Panel>
      <Panel title="Requirements">
        <ul className="hint-list">
          <li>
            Enable Wi-Fi on the VIRB 360 and join its network from this computer, or connect both
            to the same local network.
          </li>
          <li>
            When the camera acts as an access point it is usually reachable at{" "}
            <code>{view?.defaultCameraAddress ?? "192.168.0.1"}</code>; on other networks check
            your router for the camera's address.
          </li>
          <li>V360Lab only contacts the address entered here. No cloud service is used.</li>
        </ul>
      </Panel>
    </div>
  );
}
