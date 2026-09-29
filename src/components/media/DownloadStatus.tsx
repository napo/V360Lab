import type { DownloadState } from "../../context/DownloadsContext";
import { formatBytes } from "../../utils/format";
import { ErrorBanner } from "../ErrorBanner";
import { Meter } from "../Meter";

export function DownloadStatus({ state }: { state: DownloadState | undefined }) {
  if (!state) return null;

  if (state.status === "downloading") {
    const progress = state.progress;
    const fraction =
      progress?.totalBytes ? progress.receivedBytes / progress.totalBytes : null;
    return (
      <div className="download-status">
        <Meter fraction={fraction} />
        <span className="muted small mono">
          {progress
            ? `${progress.fileName}: ${formatBytes(progress.receivedBytes)}${progress.totalBytes ? ` / ${formatBytes(progress.totalBytes)}` : ""}`
            : "Starting…"}
        </span>
      </div>
    );
  }

  if (state.status === "error") {
    return <ErrorBanner error={state.error} title="Download failed" />;
  }

  const { report } = state;
  return (
    <div className="download-status">
      <span className="small ok">
        Saved {report.files.length} file{report.files.length === 1 ? "" : "s"} to{" "}
        <span className="mono">{report.directory}</span>
      </span>
      {report.files.some((f) => f.skipped) && (
        <span className="muted small">Existing complete files were kept.</span>
      )}
      {report.warnings.map((warning) => (
        <span key={warning} className="small warning">
          {warning}
        </span>
      ))}
    </div>
  );
}
