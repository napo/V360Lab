import { useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { MediaTable } from "../components/media/MediaTable";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";
import type { MediaType } from "../types/camera";
import type { DownloadOptions } from "../types/downloads";
import { formatBytes } from "../utils/format";

type Filter = "all" | MediaType;

export function MediaPage() {
  const media = useAsyncResource(cameraService.mediaList);
  const { view } = useSettings();
  const [filter, setFilter] = useState<Filter>("all");
  const [options, setOptions] = useState<DownloadOptions>({ includeFit: true, includeThumbnail: true });

  const items = useMemo(() => {
    const all = media.data ?? [];
    return all
      .filter((item) => filter === "all" || item.mediaType === filter)
      .sort((a, b) => (b.timestamp ?? 0) - (a.timestamp ?? 0));
  }, [media.data, filter]);

  const totalBytes = items.reduce((sum, item) => sum + (item.fileSizeBytes ?? 0), 0);

  return (
    <div className="page">
      <h1>Media</h1>
      <div className="toolbar">
        <button type="button" className="btn" disabled={media.loading} onClick={() => void media.reload()}>
          {media.loading ? "Loading…" : "Refresh"}
        </button>
        <select value={filter} onChange={(e) => setFilter(e.target.value as Filter)} aria-label="Media type">
          <option value="all">All media</option>
          <option value="video">Videos</option>
          <option value="photo">Photos</option>
          <option value="other">Other</option>
        </select>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={options.includeFit}
            onChange={(e) => setOptions((o) => ({ ...o, includeFit: e.target.checked }))}
          />
          Include FIT
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={options.includeThumbnail}
            onChange={(e) => setOptions((o) => ({ ...o, includeThumbnail: e.target.checked }))}
          />
          Include thumbnail
        </label>
        <span className="toolbar-spacer" />
        <span className="muted small">
          {items.length} item{items.length === 1 ? "" : "s"} · {formatBytes(totalBytes)}
        </span>
      </div>
      {view && (
        <p className="muted small">
          Downloads are saved to <span className="mono">{view.effectiveDownloadDirectory}</span>{" "}
          (change in Settings).
        </p>
      )}
      {media.error && <ErrorBanner error={media.error} title="Could not load media" onRetry={() => void media.reload()} />}
      {media.data && items.length === 0 && !media.loading && <p className="empty">No media found on the camera.</p>}
      {items.length > 0 && <MediaTable items={items} options={options} />}
    </div>
  );
}
