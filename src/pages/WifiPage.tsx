import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { Panel } from "../components/Panel";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { cameraService } from "../services/cameraService";
import { settingsService } from "../services/settingsService";
import {
  WIFI_SECURITY_TYPES,
  type WifiNetwork,
  type WifiNetworks,
  type WifiSecurity,
  type WifiSwitch,
} from "../types/camera";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";

interface WifiPageProps {
  /** Called once the camera has been asked to join `ssid`: the connection
   * is closed at that point. */
  onSwitched: (ssid: string, result: WifiSwitch) => void;
}

interface Draft {
  ssid: string;
  security: WifiSecurity;
  password: string;
  connectNow: boolean;
}

const EMPTY_DRAFT: Draft = { ssid: "", security: "WPA2", password: "", connectNow: true };

function securityLabel(network: WifiNetwork): string {
  return network.security ?? network.securityRaw ?? "?";
}

/** Networks the camera joins as a client, as in Garmin's VIRB app. */
export function WifiPage({ onSwitched }: WifiPageProps) {
  const { disconnect } = useCamera();
  const { t } = useI18n();
  const [networks, setNetworks] = useState<WifiNetworks | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);
  const [showPassword, setShowPassword] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      setNetworks(await cameraService.wifiNetworks());
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  /** Returns true once the camera has been asked to switch. */
  const connectTo = async (ssid: string): Promise<boolean> => {
    const confirmed = await settingsService.confirm(
      t("wifi.connectConfirm", { ssid, camera: networks?.accessPointSsid ?? "VIRB" }),
      { title: t("wifi.connectTitle"), okLabel: t("wifi.connect"), cancelLabel: t("wifi.cancel") },
    );
    if (!confirmed) return false;
    setBusy(ssid);
    setError(null);
    try {
      const result = await cameraService.connectWifiNetwork(ssid);
      await disconnect();
      onSwitched(ssid, result);
      return true;
    } catch (e) {
      setError(toAppError(e));
      setBusy(null);
      return false;
    }
  };

  const remove = async (ssid: string) => {
    const confirmed = await settingsService.confirm(t("wifi.removeConfirm", { ssid }), {
      title: t("wifi.removeTitle"),
      okLabel: t("wifi.remove"),
      cancelLabel: t("wifi.cancel"),
    });
    if (!confirmed) return;
    setBusy(ssid);
    setError(null);
    try {
      await cameraService.removeWifiNetwork(ssid);
      await load();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setBusy(null);
    }
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const { ssid, security, password, connectNow } = draft;
    setBusy(ssid);
    setError(null);
    setSaved(null);
    try {
      await cameraService.addWifiNetwork(ssid, security, password);
    } catch (e) {
      setError(toAppError(e));
      setBusy(null);
      return;
    }
    setSaved(ssid);
    setDraft(EMPTY_DRAFT);
    setBusy(null);
    if (connectNow && (await connectTo(ssid))) return;
    // Show the new network among the saved ones (keeping any switch error).
    try {
      setNetworks(await cameraService.wifiNetworks());
    } catch {
      /* The list is refreshed on the next reload. */
    }
  };

  const pick = (network: WifiNetwork) => {
    setDraft({ ...draft, ssid: network.ssid, security: network.security ?? "WPA2", password: "" });
    setSaved(null);
  };

  const configured = networks?.configured ?? [];
  const available = (networks?.scanned ?? []).filter(
    (n) => !configured.some((c) => c.ssid === n.ssid),
  );
  const needsPassword = draft.security !== "Open";

  return (
    <div className="page page-narrow wifi-page">
      <h1>{t("wifi.title")}</h1>
      <p className="muted">{t("wifi.intro")}</p>
      <div className="toolbar">
        <button type="button" className="btn" disabled={loading || busy !== null} onClick={() => void load()}>
          {loading ? t("wifi.scanning") : t("common.refresh")}
        </button>
      </div>
      {error && <ErrorBanner error={error} />}

      <Panel title={t("wifi.cameraNetwork")}>
        <p>
          <span className="mono">{networks?.accessPointSsid ?? "—"}</span>
        </p>
        <p className="muted small">{t("wifi.cameraNetworkHint")}</p>
      </Panel>

      <Panel title={t("wifi.saved")}>
        {configured.length === 0 ? (
          <p className="empty">{loading ? t("common.loading") : t("wifi.noneSaved")}</p>
        ) : (
          <ul className="wifi-list">
            {configured.map((network) => (
              <li key={network.ssid}>
                <span className="wifi-name">{network.ssid}</span>
                <span className="badge">{securityLabel(network)}</span>
                <span className="toolbar-spacer" />
                <button
                  type="button"
                  className="btn btn-small btn-primary"
                  disabled={busy !== null}
                  onClick={() => void connectTo(network.ssid)}
                >
                  {busy === network.ssid ? t("wifi.working") : t("wifi.connect")}
                </button>
                <button
                  type="button"
                  className="btn btn-small btn-ghost"
                  disabled={busy !== null}
                  onClick={() => void remove(network.ssid)}
                >
                  {t("wifi.remove")}
                </button>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel title={t("wifi.add")}>
        {available.length > 0 && (
          <>
            <p className="muted small">{t("wifi.availableHint")}</p>
            <ul className="wifi-list wifi-list-pick">
              {available.map((network) => (
                <li key={network.ssid}>
                  <button
                    type="button"
                    className={`wifi-pick ${draft.ssid === network.ssid ? "active" : ""}`}
                    onClick={() => pick(network)}
                  >
                    <span className="wifi-name">{network.ssid}</span>
                    <span className="badge">{securityLabel(network)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
        <form className="form" onSubmit={submit}>
          <label className="field">
            <span>{t("wifi.ssid")}</span>
            <input
              value={draft.ssid}
              onChange={(e) => setDraft({ ...draft, ssid: e.target.value })}
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              required
            />
          </label>
          <label className="field field-inline">
            <span>{t("wifi.security")}</span>
            <select
              value={draft.security}
              onChange={(e) => setDraft({ ...draft, security: e.target.value as WifiSecurity })}
            >
              {WIFI_SECURITY_TYPES.map((security) => (
                <option key={security} value={security}>
                  {security === "Open" ? t("wifi.open") : security}
                </option>
              ))}
            </select>
          </label>
          {needsPassword && (
            <label className="field">
              <span>{t("wifi.password")}</span>
              <div className="input-row">
                <input
                  type={showPassword ? "text" : "password"}
                  value={draft.password}
                  onChange={(e) => setDraft({ ...draft, password: e.target.value })}
                  autoComplete="off"
                  spellCheck={false}
                  required
                />
                <button type="button" className="btn btn-ghost" onClick={() => setShowPassword(!showPassword)}>
                  {showPassword ? t("wifi.hidePassword") : t("wifi.showPassword")}
                </button>
              </div>
            </label>
          )}
          <label className="checkbox">
            <input
              type="checkbox"
              checked={draft.connectNow}
              onChange={(e) => setDraft({ ...draft, connectNow: e.target.checked })}
            />
            {t("wifi.connectNow")}
          </label>
          <div className="form-actions">
            <button type="submit" className="btn btn-primary" disabled={busy !== null || !draft.ssid}>
              {busy === draft.ssid && draft.ssid ? t("wifi.working") : t("wifi.save")}
            </button>
            {saved && <span className="muted small">{t("wifi.savedNotice", { ssid: saved })}</span>}
          </div>
        </form>
      </Panel>
    </div>
  );
}
