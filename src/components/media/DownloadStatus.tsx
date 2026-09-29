import type { DownloadState } from "../../context/DownloadsContext";
import { useI18n } from "../../hooks/useI18n";
import { hasKey } from "../../i18n/translate";
import { useSettings } from "../../hooks/useSettings";
import { formatBytes } from "../../utils/format";
import { ErrorBanner } from "../ErrorBanner";
import { Meter } from "../Meter";

export function DownloadStatus({ state }: { state: DownloadState | undefined }) {
  const { t, tx } = useI18n();
  const { debugMode } = useSettings();
  if (!state) return null;

  if (state.status === "downloading") {
    const progress = state.progress;
    const fraction = progress?.totalBytes ? progress.receivedBytes / progress.totalBytes : null;
    return (
      <div className="download-status">
        <Meter fraction={fraction} />
        <span className="muted small mono">
          {progress
            ? `${progress.fileName}: ${formatBytes(progress.receivedBytes)}${progress.totalBytes ? ` / ${formatBytes(progress.totalBytes)}` : ""}`
            : t("download.starting")}
        </span>
      </div>
    );
  }

  if (state.status === "error") {
    return <ErrorBanner error={state.error} title={t("download.failed")} />;
  }

  const { report } = state;
  return (
    <div className="download-status">
      <span className="small ok">
        {tx("download.saved", {
          count: report.files.length,
          directory: <span className="mono">{report.directory}</span>,
        })}
      </span>
      {report.files.some((f) => f.skipped) && <span className="muted small">{t("download.kept")}</span>}
      {report.warnings.map((warning) => {
        const key = `download.warning.${warning.code}`;
        return (
          <span key={`${warning.code}-${warning.detail}`} className="small warning" title={warning.detail}>
            {t(hasKey(key) ? key : "download.warning.unknown")}
            {debugMode && <span className="mono muted"> {warning.detail}</span>}
          </span>
        );
      })}
    </div>
  );
}
