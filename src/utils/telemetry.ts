import type { TelemetrySample } from "../types/camera";

/** Index of the sample closest to `timeMs` (samples sorted by time). */
export function sampleIndexAt(samples: TelemetrySample[], timeMs: number): number {
  if (samples.length === 0) return -1;
  let low = 0;
  let high = samples.length - 1;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (samples[mid].timestampMs < timeMs) low = mid + 1;
    else high = mid;
  }
  if (low > 0 && Math.abs(samples[low - 1].timestampMs - timeMs) <= Math.abs(samples[low].timestampMs - timeMs)) {
    return low - 1;
  }
  return low;
}

export interface Point {
  x: number;
  y: number;
}

/**
 * Projects positions into a `width` × `height` box (equirectangular,
 * corrected for latitude so that shapes are not stretched), keeping the
 * aspect ratio. Samples without a position map to null.
 */
export function projectTrack(
  samples: TelemetrySample[],
  width: number,
  height: number,
  padding = 8,
): Array<Point | null> {
  const positioned = samples.filter((s) => s.latitude !== null && s.longitude !== null);
  if (positioned.length === 0) return samples.map(() => null);
  const meanLat = positioned.reduce((sum, s) => sum + s.latitude!, 0) / positioned.length;
  const scaleX = Math.cos((meanLat * Math.PI) / 180);
  const xs = positioned.map((s) => s.longitude! * scaleX);
  const ys = positioned.map((s) => s.latitude!);
  const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const spanX = Math.max(maxX - minX, 1e-9);
  const spanY = Math.max(maxY - minY, 1e-9);
  const scale = Math.min((width - 2 * padding) / spanX, (height - 2 * padding) / spanY);
  const offsetX = (width - spanX * scale) / 2;
  const offsetY = (height - spanY * scale) / 2;
  return samples.map((s) =>
    s.latitude === null || s.longitude === null
      ? null
      : { x: offsetX + (s.longitude * scaleX - minX) * scale, y: offsetY + (maxY - s.latitude) * scale },
  );
}

/** Index of the projected point nearest to (x, y). */
export function nearestPoint(points: Array<Point | null>, x: number, y: number): number {
  let best = -1;
  let bestDistance = Infinity;
  points.forEach((p, i) => {
    if (!p) return;
    const d = (p.x - x) ** 2 + (p.y - y) ** 2;
    if (d < bestDistance) {
      bestDistance = d;
      best = i;
    }
  });
  return best;
}

/** SVG polyline points for a value over video time (null values skipped). */
export function seriesPath(
  samples: TelemetrySample[],
  value: (s: TelemetrySample) => number | null,
  startMs: number,
  durationMs: number,
  width: number,
  height: number,
): string {
  const values = samples.map(value).filter((v): v is number => v !== null);
  if (values.length === 0 || durationMs <= 0) return "";
  const min = Math.min(...values);
  const span = Math.max(Math.max(...values) - min, 1e-9);
  return samples
    .map((s) => {
      const v = value(s);
      if (v === null) return null;
      const x = ((s.timestampMs - startMs) / durationMs) * width;
      const y = height - ((v - min) / span) * (height - 4) - 2;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .filter(Boolean)
    .join(" ");
}
