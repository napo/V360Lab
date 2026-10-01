import { useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { ActivityPanel } from "../components/ActivityPanel";
import { DeleteResult } from "../components/media/DeleteResult";
import { MediaGrid } from "../components/media/MediaGrid";
import { MediaViewer } from "../components/media/MediaViewer";
import { TelemetryPanel } from "../components/media/TelemetryPanel";
import { useActivity } from "../hooks/useActivity";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { useCamera } from "../hooks/useCamera";
import { useDownloads } from "../hooks/useDownloads";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";
import { settingsService } from "../services/settingsService";
import type { MediaItem, MediaType } from "../types/camera";
import type { DeleteReport, DownloadOptions } from "../types/downloads";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";
import { formatBytes } from "../utils/format";

type Filter = "all" | "favorites" | MediaType;

export function MediaPage() {
  const loading = useActivity();
  const deletion = useActivity();
  const media = useAsyncResource(async () => {
    loading.start();
    loading.step("mediaRequest");
    try {
      const items = await cameraService.mediaList();
      loading.step("mediaReceived", { count: items.length }, "success");
      loading.finish();
      return items;
    } catch (e) {
      loading.fail(toAppError(e));
      throw e;
    }
  });
  const { view } = useSettings();
  const { status, supports } = useCamera();
  const { start } = useDownloads();
  const { t, tx } = useI18n();
  const [filter, setFilter] = useState<Filter>("all");
  const [options, setOptions] = useState<DownloadOptions>({ includeFit: true, includeThumbnail: true });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [deleteReport, setDeleteReport] = useState<DeleteReport | null>(null);
  const [deleteError, setDeleteError] = useState<AppError | null>(null);
  const [favoriteError, setFavoriteError] = useState<AppError | null>(null);
  const [favoriteBusy, setFavoriteBusy] = useState<string | null>(null);
  const [viewing, setViewing] = useState<MediaItem | null>(null);

  const recording = status?.recordingState === "recording";

  const items = useMemo(() => {
    const all = media.data ?? [];
    return all
      .filter(
        (item) =>
          filter === "all" || (filter === "favorites" ? item.favorite === true : item.mediaType === filter),
      )
      .sort((a, b) => (b.timestamp ?? 0) - (a.timestamp ?? 0));
  }, [media.data, filter]);

  const selectedItems = useMemo(
    () => (media.data ?? []).filter((item) => selected.has(item.id)),
    [media.data, selected],
  );
  const totalBytes = items.reduce((sum, item) => sum + (item.fileSizeBytes ?? 0), 0);

  const toggleSelected = (item: MediaItem) =>
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(item.id)) next.delete(item.id);
      else next.add(item.id);
      return next;
    });

  const toggleAll = () =>
    setSelected((current) => {
      const allVisible = items.length > 0 && items.every((item) => current.has(item.id));
      const next = new Set(current);
      for (const item of items) {
        if (allVisible) next.delete(item.id);
        else next.add(item.id);
      }
      return next;
    });

  // One download at a time: the camera's HTTP server is slow.
  const downloadSelected = async () => {
    for (const item of selectedItems) {
      if (item.url) await start(item, "media", options);
    }
  };

  const toggleFavorite = async (item: MediaItem) => {
    setFavoriteBusy(item.id);
    setFavoriteError(null);
    try {
      const updated = await cameraService.setFavorite(item, !item.favorite);
      media.update((all) => all?.map((m) => (m.id === updated.id ? updated : m)) ?? all);
    } catch (e) {
      setFavoriteError(toAppError(e));
    } finally {
      setFavoriteBusy(null);
    }
  };

  const deleteItems = async (targets: MediaItem[]) => {
    if (targets.length === 0) return;
    const confirmed = await settingsService.confirm(t("media.deleteConfirm", { count: targets.length }), {
      title: t("media.deleteTitle"),
      okLabel: t("common.delete"),
      cancelLabel: t("common.cancel"),
    });
    if (!confirmed) return;
    setDeleting(true);
    setDeleteReport(null);
    setDeleteError(null);
    const activityId = deletion.start();
    try {
      const report = await cameraService.deleteMedia(targets, activityId);
      deletion.finish();
      setDeleteReport(report);
      setSelected((current) => {
        const next = new Set(current);
        for (const id of report.deleted) next.delete(id);
        return next;
      });
      await media.reload();
    } catch (e) {
      const error = toAppError(e);
      setDeleteError(error);
      deletion.fail(error);
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="page">
      <h1>{t("media.title")}</h1>
      <div className="toolbar">
        <button type="button" className="btn" disabled={media.loading} onClick={() => void media.reload()}>
          {media.loading ? t("common.loading") : t("common.refresh")}
        </button>
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value as Filter)}
          aria-label={t("media.filterLabel")}
        >
          <option value="all">{t("media.filterAll")}</option>
          <option value="video">{t("media.filterVideo")}</option>
          <option value="photo">{t("media.filterPhoto")}</option>
          <option value="other">{t("media.filterOther")}</option>
          <option value="favorites">{t("media.filterFavorites")}</option>
        </select>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={options.includeFit}
            onChange={(e) => setOptions((o) => ({ ...o, includeFit: e.target.checked }))}
          />
          {t("media.includeFit")}
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={options.includeThumbnail}
            onChange={(e) => setOptions((o) => ({ ...o, includeThumbnail: e.target.checked }))}
          />
          {t("media.includeThumbnail")}
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={items.length > 0 && items.every((item) => selected.has(item.id))}
            onChange={toggleAll}
          />
          {t("media.selectAll")}
        </label>
        <span className="toolbar-spacer" />
        <span className="muted small">
          {t("media.count", { count: items.length, size: formatBytes(totalBytes) })}
        </span>
      </div>

      {selectedItems.length > 0 && (
        <div className="selection-bar">
          <strong>{t("media.selected", { count: selectedItems.length })}</strong>
          <button type="button" className="btn btn-small btn-primary" onClick={() => void downloadSelected()}>
            {t("media.downloadSelected")}
          </button>
          <button
            type="button"
            className="btn btn-small btn-danger"
            disabled={deleting || recording}
            title={recording ? t("media.recordingNoDelete") : undefined}
            onClick={() => void deleteItems(selectedItems)}
          >
            {t("media.deleteSelected")}
          </button>
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setSelected(new Set())}>
            {t("media.clearSelection")}
          </button>
        </div>
      )}

      {view && (
        <p className="muted small">
          {tx("media.savedTo", {
            directory: <span className="mono">{view.effectiveDownloadDirectory}</span>,
          })}
        </p>
      )}
      <ActivityPanel activity={deletion.state} title={t("activity.titleDelete")} />
      {deleteReport && <DeleteResult report={deleteReport} />}
      {favoriteError && <ErrorBanner error={favoriteError} title={t("media.favoriteFailed")} />}
      {deleteError && !deletion.state.error && (
        <ErrorBanner error={deleteError} title={t("media.deleteTitle")} />
      )}
      <ActivityPanel activity={loading.state} title={t("activity.titleMedia")} collapseWhenDone />
      {media.error && (
        <button type="button" className="btn btn-small" onClick={() => void media.reload()}>
          {t("common.retry")}
        </button>
      )}
      {media.data && items.length === 0 && !media.loading && <p className="empty">{t("media.empty")}</p>}
      {items.length > 0 && (
        <MediaGrid
          items={items}
          options={options}
          selected={selected}
          onToggleSelected={toggleSelected}
          onDelete={(item) => void deleteItems([item])}
          deleteDisabled={deleting || recording}
          onToggleFavorite={supports("setFavorite") ? (item) => void toggleFavorite(item) : undefined}
          favoriteBusy={favoriteBusy}
          onOpen={setViewing}
        />
      )}
      {viewing && (
        <MediaViewer item={viewing} onClose={() => setViewing(null)}>
          {(time, seek) =>
            viewing.mediaType === "video" && viewing.hasFit ? (
              <TelemetryPanel item={viewing} time={time} seek={seek} />
            ) : null
          }
        </MediaViewer>
      )}
    </div>
  );
}
