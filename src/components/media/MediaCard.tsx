import { useState } from "react";
import { useDownloads } from "../../hooks/useDownloads";
import { useI18n } from "../../hooks/useI18n";
import { useSettings } from "../../hooks/useSettings";
import { optionLabel } from "../../i18n/featureLabels";
import type { MediaItem } from "../../types/camera";
import type { DownloadOptions } from "../../types/downloads";
import { formatBytes, formatDateTime, formatDuration } from "../../utils/format";
import { JsonViewer } from "../JsonViewer";
import { KeyValueList } from "../KeyValueList";
import { DownloadStatus } from "./DownloadStatus";
import { Thumbnail } from "./Thumbnail";

interface MediaCardProps {
  item: MediaItem;
  options: DownloadOptions;
  selected: boolean;
  onToggleSelected: (item: MediaItem) => void;
  onDelete: (item: MediaItem) => void;
  /** Deleting is disabled while recording or another deletion runs. */
  deleteDisabled: boolean;
}

/** One media file as a card: thumbnail, key facts and actions. */
export function MediaCard({ item, options, selected, onToggleSelected, onDelete, deleteDisabled }: MediaCardProps) {
  const [expanded, setExpanded] = useState(false);
  const { downloads, start } = useDownloads();
  const { debugMode } = useSettings();
  const { t, lookup } = useI18n();
  const download = downloads[item.id];
  const busy = download?.status === "downloading";
  const lens = item.lensMode ? optionLabel(lookup, "video360Format", item.lensMode) : null;

  return (
    <article className={`media-card ${selected ? "selected" : ""}`}>
      <div className="media-thumb">
        <Thumbnail url={item.thumbnailUrl} mediaType={item.mediaType} />
        <label className="media-select">
          <input
            type="checkbox"
            checked={selected}
            aria-label={t("media.select")}
            onChange={() => onToggleSelected(item)}
          />
        </label>
        <div className="media-badges">
          {item.mediaType === "video" && item.durationSecs != null && (
            <span className="badge-pill">{formatDuration(item.durationSecs)}</span>
          )}
          {item.hasFit && <span className="badge-pill badge-fit">FIT</span>}
        </div>
      </div>
      <div className="media-info">
        <div className="mono media-name">{item.name}</div>
        <div className="muted small">
          {formatDateTime(item.dateTime)} · {formatBytes(item.fileSizeBytes)}
        </div>
        {lens && <div className="small">{lens}</div>}
      </div>
      <div className="media-actions">
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
          {expanded ? t("media.hide") : t("media.cardDetails")}
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
      {expanded && (
        <div className="media-details">
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
        </div>
      )}
    </article>
  );
}
