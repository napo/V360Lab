import { useEffect, useState } from "react";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { isTauri } from "../services/backend";
import { releaseHighlights, updateService } from "../services/updateService";
import type { AppError } from "../types/errors";
import type { UpdateInfo } from "../types/settings";
import { toAppError } from "../utils/errors";
import { formatBytes } from "../utils/format";
import { ErrorBanner } from "./ErrorBanner";

type Install =
  | { status: "idle" }
  | { status: "installing"; received: number; total: number | null }
  | { status: "downloadStarted" };

/**
 * Tells about a newer release at start-up and installs it once the user
 * agrees: on desktop the signed update is downloaded, installed and the
 * app restarts; on Android the APK is downloaded and Android asks to
 * install it.
 */
export function UpdateNotice() {
  const { t } = useI18n();
  const { view } = useSettings();
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [install, setInstall] = useState<Install>({ status: "idle" });
  const [error, setError] = useState<AppError | null>(null);
  const enabled = view?.settings.checkUpdates ?? false;

  useEffect(() => {
    if (!enabled || !isTauri() || !updateService.dueAtStartup()) return;
    let cancelled = false;
    updateService
      .check()
      .then((info) => {
        updateService.markChecked();
        if (!cancelled && info.available && !updateService.isSkipped(info.latestVersion)) setUpdate(info);
      })
      // Offline or GitHub unreachable: try again at the next start.
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  if (!update) return null;

  const installNow = async () => {
    setError(null);
    try {
      if (update.platform === "android") {
        if (!update.apkUrl) throw new Error("no APK in the release");
        await updateService.open(update.apkUrl);
        setInstall({ status: "downloadStarted" });
        return;
      }
      setInstall({ status: "installing", received: 0, total: null });
      const installed = await updateService.installDesktop((received, total) =>
        setInstall({ status: "installing", received, total }),
      );
      // No signed update for this kind of installation: open the page.
      if (!installed) {
        await updateService.open(update.pageUrl);
        setInstall({ status: "downloadStarted" });
      }
    } catch (e) {
      setError(toAppError(e));
      setInstall({ status: "idle" });
    }
  };

  const highlights = releaseHighlights(update.notes);

  return (
    <aside className="update-notice" role="status">
      <strong>{t("update.available", { version: update.latestVersion, current: update.currentVersion })}</strong>
      {highlights.length > 0 && (
        <ul className="update-highlights">
          {highlights.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
      {install.status === "installing" ? (
        <div className="frame-progress">
          <progress value={install.received} max={install.total ?? undefined} />
          <span className="mono small">
            {formatBytes(install.received)}
            {install.total ? ` / ${formatBytes(install.total)}` : ""}
          </span>
        </div>
      ) : install.status === "downloadStarted" ? (
        <p className="small">
          {update.platform === "android" ? t("update.androidInstall") : t("update.manualInstall")}
        </p>
      ) : (
        <div className="form-actions">
          <button type="button" className="btn btn-small btn-primary" onClick={() => void installNow()}>
            {update.platform === "android" ? t("update.downloadApk") : t("update.installNow")}
          </button>
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setUpdate(null)}>
            {t("update.later")}
          </button>
          <button
            type="button"
            className="btn btn-small btn-ghost"
            onClick={() => {
              updateService.skip(update.latestVersion);
              setUpdate(null);
            }}
          >
            {t("update.skip")}
          </button>
          <button type="button" className="btn btn-small btn-ghost" onClick={() => void updateService.open(update.pageUrl)}>
            {t("update.details")}
          </button>
        </div>
      )}
      {update.platform === "desktop" && install.status === "idle" && (
        <p className="muted small">{t("update.desktopHint")}</p>
      )}
      {error && (
        <>
          <ErrorBanner error={error} title={t("update.failed")} />
          <button type="button" className="btn btn-small" onClick={() => void updateService.open(update.pageUrl)}>
            {t("update.openPage")}
          </button>
        </>
      )}
    </aside>
  );
}
