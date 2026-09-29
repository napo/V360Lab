import { useState } from "react";
import { useDownloads } from "../../hooks/useDownloads";
import { useI18n } from "../../hooks/useI18n";
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
  selected: boolean;
  onToggleSelected: (item: MediaItem) => void;
  onDelete: (item: MediaItem) => void;
  /** Deleting is disabled while recording or another deletion runs. */
  deleteDisabled: boolean;
}

export const MEDIA_COLUMN_COUNT = 9;

export function MediaRow({
  item,
  options,
  selected,
  onToggleSelected,
  onDelete,
  deleteDisabled,
}: MediaRowProps) {
  const [expanded, setExpanded] = useState(false);
  const { downloads, start } = useDownloads();
  const { debugMode } = useSettings();
  const { t } = useI18n();
  const download = downloads[item.id];
  const busy = download?.status === "downloading";

  return (
    <>
      <tr className={expanded ? "expanded" : undefined}>
        <td className="checkbox-cell">
          <input
            type="checkbox"
            checked={selected}
            aria-label={t("media.select")}
            onChange={() => onToggleSelected(item)}
          />
        </td>
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
              title={item.url ? undefined : t("media.noUrl")}
              onClick={() => void start(item, "media", options)}
            >
              {t("media.download")}
            </button>
            <button
              type="button"
              className="btn btn-small"
              disabled={busy || !item.hasFit}
              title={item.hasFit ? t("media.fitOnly") : t("media.noFit")}
              onClick={() => void start(item, "fit", options)}
            >
              {t("media.fit")}
            </button>
            <button
              type="button"
              className="btn btn-small btn-ghost"
              aria-expanded={expanded}
              onClick={() => setExpanded((v) => !v)}
            >
              {expanded ? t("media.hide") : t("media.details")}
            </button>
            <button
              type="button"
              className="btn btn-small btn-danger"
              disabled={busy || deleteDisabled}
              onClick={() => onDelete(item)}
            >
              {t("media.delete")}
            </button>
          </div>
          <DownloadStatus state={download} />
        </td>
      </tr>
      {expanded && (
        <tr className="details-row">
          <td colSpan={MEDIA_COLUMN_COUNT}>
            <KeyValueList
              items={[
                { label: t("media.mediaUrl"), value: item.url, mono: true },
                { label: t("media.lowResUrl"), value: item.lowResUrl, mono: true },
                { label: t("media.thumbnailUrl"), value: item.thumbnailUrl, mono: true },
                { label: t("media.fitUrl"), value: item.fitUrl, mono: true },
                { label: t("media.timestampUtc"), value: item.dateTime, mono: true },
              ]}
            />
            <JsonViewer value={item.raw} title={t("media.cameraMetadata")} defaultOpen={debugMode} />
          </td>
        </tr>
      )}
    </>
  );
}
