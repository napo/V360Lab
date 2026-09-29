import { useState } from "react";
import { useDownloads } from "../../hooks/useDownloads";
import { useSettings } from "../../hooks/useSettings";
import type { MediaItem } from "../../types/camera";
import type { DownloadOptions } from "../../types/downloads";
import { formatBytes, formatDateTime, formatDuration } from "../../utils/format";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { DownloadStatus } from "./DownloadStatus";
import { Thumbnail } from "./Thumbnail";

interface MediaRowProps {
  item: MediaItem;
  options: DownloadOptions;
}

const COLUMN_COUNT = 8;

export function MediaRow({ item, options }: MediaRowProps) {
  const [expanded, setExpanded] = useState(false);
  const { downloads, start } = useDownloads();
  const { debugMode } = useSettings();
  const download = downloads[item.id];
  const busy = download?.status === "downloading";

  return (
    <>
      <tr className={expanded ? "expanded" : undefined}>
        <td>
          <Thumbnail url={item.thumbnailUrl} mediaType={item.mediaType} />
        </td>
        <td>
          <div className="mono">{item.name}</div>
          <span className={`tag tag-${item.mediaType}`}>{item.mediaTypeRaw ?? item.mediaType}</span>
        </td>
        <td className="mono nowrap">{formatDateTime(item.dateTime)}</td>
        <td className="mono">{item.mediaType === "photo" ? "" : formatDuration(item.durationSecs)}</td>
        <td className="mono nowrap">{formatBytes(item.fileSizeBytes)}</td>
        <td>{item.lensMode ?? ""}</td>
        <td>{item.hasFit ? <span className="tag tag-fit">FIT</span> : <span className="muted">—</span>}</td>
        <td className="actions-cell">
          <div className="row-actions">
            <button
              type="button"
              className="btn btn-small btn-primary"
              disabled={busy || !item.url}
              title={item.url ? undefined : "The camera reported no download URL"}
              onClick={() => void start(item, "media", options)}
            >
              Download
            </button>
            <button
              type="button"
              className="btn btn-small"
              disabled={busy || !item.hasFit}
              title={item.hasFit ? "Download only the FIT telemetry file" : "No FIT file associated"}
              onClick={() => void start(item, "fit", options)}
            >
              FIT
            </button>
            <button
              type="button"
              className="btn btn-small btn-ghost"
              aria-expanded={expanded}
              onClick={() => setExpanded((v) => !v)}
            >
              {expanded ? "Hide" : "Details"}
            </button>
          </div>
          <DownloadStatus state={download} />
        </td>
      </tr>
      {expanded && (
        <tr className="details-row">
          <td colSpan={COLUMN_COUNT}>
            <KeyValueList
              items={[
                { label: "Media URL", value: item.url, mono: true },
                { label: "Low-res preview URL", value: item.lowResUrl, mono: true },
                { label: "Thumbnail URL", value: item.thumbnailUrl, mono: true },
                { label: "FIT URL", value: item.fitUrl, mono: true },
                { label: "Timestamp (UTC)", value: item.dateTime, mono: true },
              ]}
            />
            <JsonViewer value={item.raw} title="Camera metadata" defaultOpen={debugMode} />
          </td>
        </tr>
      )}
    </>
  );
}
