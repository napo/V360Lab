import { useEffect, useState, type FormEvent } from "react";
import { ActivityPanel } from "../components/ActivityPanel";
import { BrandHero } from "../components/BrandHero";
import { Panel } from "../components/Panel";
import { useActivity } from "../hooks/useActivity";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";
import type { DiscoveredCamera } from "../types/camera";
import { toAppError } from "../utils/errors";

/**
 * Shown instead of camera pages while no camera is connected. The main
 * action is a single "find and connect" button; manual entry and the mock
 * camera are secondary.
 */
export function ConnectionPage() {
  const { connection, connect } = useCamera();
  const { view } = useSettings();
  const { t, tx } = useI18n();
  const activity = useActivity();
  const [found, setFound] = useState<DiscoveredCamera[]>([]);
  const [phase, setPhase] = useState<"discover" | "connect">("discover");
  const [address, setAddress] = useState("");
  const [mock, setMock] = useState(false);
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    if (!view || initialized) return;
    setAddress(view.settings.lastCameraAddress ?? view.defaultCameraAddress);
    setMock(view.settings.mockMode);
    setInitialized(true);
  }, [view, initialized]);

  const busy = activity.state.status === "running" || connection.status === "connecting";
  const defaultAddress = view?.defaultCameraAddress ?? "192.168.0.1";

  const connectTo = async (target: string, useMock: boolean, activityId: string) => {
    setPhase("connect");
    const ok = await connect(target, useMock, activityId);
    if (ok) activity.finish();
  };

  // Reports connection failures (kept in the camera context) in the panel.
  const { fail } = activity;
  const activityStatus = activity.state.status;
  useEffect(() => {
    if (connection.status === "error" && activityStatus === "running") fail(connection.error);
  }, [connection, activityStatus, fail]);

  const findAndConnect = async () => {
    setFound([]);
    setPhase("discover");
    const id = activity.start();
    try {
      const cameras = await cameraService.discover(id);
      if (cameras.length === 1) {
        await connectTo(cameras[0].address, false, id);
      } else if (cameras.length > 1) {
        activity.step("discoveryChoose");
        activity.finish();
        setFound(cameras);
      } else {
        activity.finish();
      }
    } catch (e) {
      activity.fail(toAppError(e));
    }
  };

  const connectManually = (event: FormEvent) => {
    event.preventDefault();
    if (busy) return;
    setFound([]);
    void connectTo(address, mock, activity.start());
  };

  return (
    <div className="page page-narrow">
      <BrandHero />

      <div className="connect-main">
        <button
          type="button"
          className="btn btn-primary btn-hero"
          disabled={busy}
          onClick={() => void findAndConnect()}
        >
          {busy ? t("connect.searching") : t("connect.findAndConnect")}
        </button>
        <p className="muted small">{t("connect.findHint")}</p>
      </div>

      <ActivityPanel
        activity={activity.state}
        title={t(phase === "discover" ? "activity.titleDiscover" : "activity.titleConnect")}
      />

      {found.length > 0 && (
        <Panel title={t("connect.foundCameras")}>
          <ul className="camera-choices">
            {found.map((camera) => (
              <li key={camera.address}>
                <button
                  type="button"
                  className="btn"
                  disabled={busy}
                  onClick={() => void connectTo(camera.address, false, activity.start())}
                >
                  <strong>{camera.model ?? "VIRB"}</strong>
                  <span className="mono">{camera.address}</span>
                  {camera.firmware && <span className="muted small">fw {camera.firmware}</span>}
                </button>
              </li>
            ))}
          </ul>
        </Panel>
      )}

      <details className="panel connect-manual">
        <summary className="panel-header">
          <h2>{t("connect.manual")}</h2>
        </summary>
        <div className="panel-body">
          <form className="form" onSubmit={connectManually}>
            <label className="field">
              <span>{t("connect.addressLabel")}</span>
              <input
                className="mono"
                value={address}
                onChange={(e) => setAddress(e.target.value)}
                placeholder={defaultAddress}
                disabled={busy || mock}
                spellCheck={false}
              />
            </label>
            <label className="checkbox">
              <input type="checkbox" checked={mock} onChange={(e) => setMock(e.target.checked)} disabled={busy} />
              {t("connect.useMock")}
            </label>
            <div className="form-actions">
              <button type="submit" className="btn" disabled={busy || (!mock && !address.trim())}>
                {t("connect.connect")}
              </button>
            </div>
          </form>
        </div>
      </details>

      <details className="panel">
        <summary className="panel-header">
          <h2>{t("connect.requirements")}</h2>
        </summary>
        <div className="panel-body">
          <ul className="hint-list">
            <li>{t("connect.requirementWifi")}</li>
            <li>{t("connect.requirementPassword")}</li>
            <li>{tx("connect.requirementAddress", { address: <code>{defaultAddress}</code> })}</li>
            <li>{t("connect.requirementLocal")}</li>
          </ul>
        </div>
      </details>
    </div>
  );
}
