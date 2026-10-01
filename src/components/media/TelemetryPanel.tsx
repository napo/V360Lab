import { useMemo, useState, type MouseEvent } from "react";
import { useI18n } from "../../hooks/useI18n";
import type { TelemetryState } from "../../hooks/useVideoTelemetry";
import { cameraService } from "../../services/cameraService";
import type { MediaItem } from "../../types/camera";
import type { AppError } from "../../types/errors";
import { toAppError } from "../../utils/errors";
import { formatDuration } from "../../utils/format";
import { accelAt, detectAxes, tiltAngles, tiltSeries, upInImage } from "../../utils/level";
import { nearestPoint, projectTrack, sampleIndexAt, seriesPath } from "../../utils/telemetry";
import { ErrorBanner } from "../ErrorBanner";
import { FrameExtractor } from "./FrameExtractor";

const MAP_W = 320;
const MAP_H = 220;
const CHART_W = 600;
const CHART_H = 70;
/** The tilt chart shows ±TILT_RANGE degrees. */
const TILT_RANGE = 45;

interface TelemetryPanelProps {
  item: MediaItem;
  /** Playback position in seconds. */
  time: number;
  seek: (seconds: number) => void;
  telemetry: TelemetryState;
}

function svgPoint(e: MouseEvent<SVGSVGElement>, width: number, height: number) {
  const box = e.currentTarget.getBoundingClientRect();
  return {
    x: ((e.clientX - box.left) / box.width) * width,
    y: ((e.clientY - box.top) / box.height) * height,
  };
}

/** GPS track, speed and altitude of a video from its FIT file, following
 * the playback: the marker moves with the video, a tap seeks to that spot. */
export function TelemetryPanel({ item, time, seek, telemetry: state }: TelemetryPanelProps) {
  const { t } = useI18n();
  const telemetry = state.data;
  const error = state.error;
  const [exporting, setExporting] = useState(false);
  const [exported, setExported] = useState<string | null>(null);
  const [exportError, setExportError] = useState<AppError | null>(null);

  const exportTrack = async (format: "gpx" | "geojson") => {
    setExporting(true);
    setExported(null);
    setExportError(null);
    try {
      setExported(await cameraService.exportTrack(item, format));
    } catch (e) {
      setExportError(toAppError(e));
    } finally {
      setExporting(false);
    }
  };

  const axes = useMemo(() => detectAxes(telemetry?.accelerometer ?? []), [telemetry]);
  const points = useMemo(
    () => (telemetry ? projectTrack(telemetry.samples, MAP_W, MAP_H) : []),
    [telemetry],
  );
  const durationMs = Math.max(
    (item.durationSecs ?? 0) * 1000,
    telemetry && telemetry.samples.length > 0
      ? telemetry.samples[telemetry.samples.length - 1].timestampMs - telemetry.videoStartMs
      : 0,
  );
  const tilt = useMemo(() => {
    if (!telemetry || !axes) return null;
    const series = tiltSeries(telemetry.accelerometer, axes);
    if (series.length === 0) return null;
    const path = (value: (p: (typeof series)[number]) => number) =>
      series
        .map((p) => {
          const x = ((p.timestampMs - telemetry.videoStartMs) / Math.max(durationMs, 1)) * CHART_W;
          const v = Math.max(-TILT_RANGE, Math.min(TILT_RANGE, value(p)));
          const y = CHART_H / 2 - (v / TILT_RANGE) * (CHART_H / 2 - 2);
          return `${x.toFixed(1)},${y.toFixed(1)}`;
        })
        .join(" ");
    return { roll: path((p) => p.rollDeg), pitch: path((p) => p.pitchDeg) };
  }, [telemetry, axes, durationMs]);
  const charts = useMemo(() => {
    if (!telemetry) return null;
    const { samples, videoStartMs } = telemetry;
    return {
      speed: seriesPath(samples, (s) => s.speedMps, videoStartMs, durationMs, CHART_W, CHART_H),
      altitude: seriesPath(samples, (s) => s.altitudeM, videoStartMs, durationMs, CHART_W, CHART_H),
    };
  }, [telemetry, durationMs]);

  if (error) return <ErrorBanner error={error} title={t("telemetry.failed")} />;
  if (!telemetry) return <p className="muted small">{t("telemetry.loading")}</p>;
  if (telemetry.samples.length === 0) return <p className="muted small">{t("telemetry.empty")}</p>;

  const { samples, summary, videoStartMs } = telemetry;
  const reading = axes ? accelAt(telemetry.accelerometer, videoStartMs + time * 1000) : null;
  const upNow = reading && axes ? upInImage(reading, axes) : null;
  const tiltNow = upNow ? tiltAngles(upNow) : null;
  const current = sampleIndexAt(samples, videoStartMs + time * 1000);
  const sampleNow = current >= 0 ? samples[current] : null;
  const marker = current >= 0 ? points[current] : null;
  const track = points
    .filter((p): p is NonNullable<typeof p> => p !== null)
    .map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(" ");
  const playhead = durationMs > 0 ? (time * 1000 * CHART_W) / durationMs : 0;
  const kmh = (mps: number | null | undefined) => (mps == null ? "—" : `${(mps * 3.6).toFixed(1)} km/h`);
  const metres = (m: number | null | undefined) => (m == null ? "—" : `${Math.round(m)} m`);

  const seekToSample = (index: number) => {
    if (index >= 0) seek((samples[index].timestampMs - videoStartMs) / 1000);
  };

  return (
    <section className="telemetry" aria-label={t("telemetry.title")}>
      <h2>{t("telemetry.title")}</h2>
      <dl className="telemetry-stats">
        <div>
          <dt>{t("telemetry.now")}</dt>
          <dd>
            {kmh(sampleNow?.speedMps)} · {metres(sampleNow?.altitudeM)}
          </dd>
        </div>
        {tiltNow && (
          <div>
            <dt>{t("telemetry.tilt")}</dt>
            <dd>{t("telemetry.tiltValue", { roll: tiltNow.rollDeg.toFixed(0), pitch: tiltNow.pitchDeg.toFixed(0) })}</dd>
          </div>
        )}
        <div>
          <dt>{t("telemetry.distance")}</dt>
          <dd>{(summary.distanceM / 1000).toFixed(2)} km</dd>
        </div>
        <div>
          <dt>{t("telemetry.maxSpeed")}</dt>
          <dd>{kmh(summary.maxSpeedMps)}</dd>
        </div>
        <div>
          <dt>{t("telemetry.avgSpeed")}</dt>
          <dd>{kmh(summary.avgSpeedMps)}</dd>
        </div>
        <div>
          <dt>{t("telemetry.altitude")}</dt>
          <dd>
            {metres(summary.minAltitudeM)} – {metres(summary.maxAltitudeM)}
          </dd>
        </div>
        <div>
          <dt>{t("telemetry.climb")}</dt>
          <dd>{metres(summary.elevationGainM)}</dd>
        </div>
      </dl>

      <div className="telemetry-views">
        {summary.hasPosition && (
          <svg
            className="telemetry-map"
            viewBox={`0 0 ${MAP_W} ${MAP_H}`}
            role="img"
            aria-label={t("telemetry.track")}
            onClick={(e) => {
              const { x, y } = svgPoint(e, MAP_W, MAP_H);
              seekToSample(nearestPoint(points, x, y));
            }}
          >
            <polyline points={track} className="telemetry-line" />
            {marker && <circle cx={marker.x} cy={marker.y} r={6} className="telemetry-marker" />}
          </svg>
        )}
        <svg
          className="telemetry-chart"
          viewBox={`0 0 ${CHART_W} ${CHART_H}`}
          preserveAspectRatio="none"
          role="img"
          aria-label={t("telemetry.chart")}
          onClick={(e) => {
            const { x } = svgPoint(e, CHART_W, CHART_H);
            seek(((x / CHART_W) * durationMs) / 1000);
          }}
        >
          {charts?.altitude && <polyline points={charts.altitude} className="telemetry-altitude" />}
          {charts?.speed && <polyline points={charts.speed} className="telemetry-speed" />}
          <line x1={playhead} x2={playhead} y1={0} y2={CHART_H} className="telemetry-playhead" />
        </svg>
        <p className="muted small telemetry-legend">
          <span className="legend-speed">{t("telemetry.speed")}</span>
          <span className="legend-altitude">{t("telemetry.altitudeLine")}</span>
          <span>{formatDuration(durationMs / 1000)}</span>
          {!telemetry.startFromCameraEvent && <span>{t("telemetry.approximateSync")}</span>}
          {summary.droppedPositions > 0 && (
            <span>{t("telemetry.droppedPositions", { count: summary.droppedPositions })}</span>
          )}
        </p>
        {tilt && (
          <>
            <svg
              className="telemetry-chart"
              viewBox={`0 0 ${CHART_W} ${CHART_H}`}
              preserveAspectRatio="none"
              role="img"
              aria-label={t("telemetry.tiltChart")}
              onClick={(e) => {
                const { x } = svgPoint(e, CHART_W, CHART_H);
                seek(((x / CHART_W) * durationMs) / 1000);
              }}
            >
              <line x1={0} x2={CHART_W} y1={CHART_H / 2} y2={CHART_H / 2} className="telemetry-zero" />
              <polyline points={tilt.roll} className="telemetry-roll" />
              <polyline points={tilt.pitch} className="telemetry-pitch" />
              <line x1={playhead} x2={playhead} y1={0} y2={CHART_H} className="telemetry-playhead" />
            </svg>
            <p className="muted small telemetry-legend">
              <span className="legend-roll">{t("telemetry.roll")}</span>
              <span className="legend-pitch">{t("telemetry.pitch")}</span>
              <span>±{TILT_RANGE}°</span>
            </p>
          </>
        )}
      </div>
      {summary.hasPosition && (
        <div className="form-actions">
          <button type="button" className="btn btn-small" disabled={exporting} onClick={() => void exportTrack("gpx")}>
            {t("telemetry.exportGpx")}
          </button>
          <button
            type="button"
            className="btn btn-small"
            disabled={exporting}
            onClick={() => void exportTrack("geojson")}
          >
            {t("telemetry.exportGeojson")}
          </button>
          {exported && (
            <span className="muted small">
              {t("telemetry.exported")} <span className="mono selectable">{exported}</span>
            </span>
          )}
        </div>
      )}
      {exportError && <ErrorBanner error={exportError} title={t("telemetry.exportFailed")} />}
      {summary.hasPosition && <FrameExtractor item={item} telemetry={telemetry} />}
    </section>
  );
}
