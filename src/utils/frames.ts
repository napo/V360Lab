import type { TelemetrySample } from "../types/camera";

/** Where and when a frame is taken (sent to the backend for EXIF). */
export interface FrameLocation {
  unixMs: number;
  latitude: number;
  longitude: number;
  altitudeM: number | null;
  headingDeg: number | null;
  speedMps: number | null;
}

export interface PlannedFrame {
  /** Position in the video, in seconds. */
  videoSeconds: number;
  location: FrameLocation;
}

export type FrameSpacing = { kind: "distance"; meters: number } | { kind: "time"; seconds: number };

const EARTH_RADIUS_M = 6_371_008.8;
const RAD = Math.PI / 180;

export function distanceM(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const dp = (lat2 - lat1) * RAD;
  const dl = (lon2 - lon1) * RAD;
  const a = Math.sin(dp / 2) ** 2 + Math.cos(lat1 * RAD) * Math.cos(lat2 * RAD) * Math.sin(dl / 2) ** 2;
  return 2 * EARTH_RADIUS_M * Math.asin(Math.sqrt(a));
}

/** Initial bearing from point 1 to point 2, degrees clockwise from north. */
export function bearingDeg(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const p1 = lat1 * RAD;
  const p2 = lat2 * RAD;
  const dl = (lon2 - lon1) * RAD;
  const y = Math.sin(dl) * Math.cos(p2);
  const x = Math.cos(p1) * Math.sin(p2) - Math.sin(p1) * Math.cos(p2) * Math.cos(dl);
  return ((Math.atan2(y, x) / RAD) + 360) % 360;
}

type Fix = TelemetrySample & { latitude: number; longitude: number };

const lerp = (a: number, b: number, f: number) => a + (b - a) * f;
const lerpNullable = (a: number | null, b: number | null, f: number) =>
  a === null || b === null ? (a ?? b) : lerp(a, b, f);

/** Location between two fixes at fraction `f`; the heading comes from the
 * GPS when known, otherwise from the direction of travel. */
function between(a: Fix, b: Fix, f: number): FrameLocation {
  const moved = distanceM(a.latitude, a.longitude, b.latitude, b.longitude) > 0.5;
  return {
    unixMs: Math.round(lerp(a.timestampMs, b.timestampMs, f)),
    latitude: lerp(a.latitude, b.latitude, f),
    longitude: lerp(a.longitude, b.longitude, f),
    altitudeM: lerpNullable(a.altitudeM, b.altitudeM, f),
    headingDeg: a.headingDeg ?? (moved ? bearingDeg(a.latitude, a.longitude, b.latitude, b.longitude) : null),
    speedMps: lerpNullable(a.speedMps, b.speedMps, f),
  };
}

/**
 * Frames to extract from a video, every `spacing` metres travelled or
 * seconds, within the video (`durationMs` from `videoStartMs`).
 */
export function planFrames(
  samples: TelemetrySample[],
  videoStartMs: number,
  durationMs: number,
  spacing: FrameSpacing,
  maxFrames = 2000,
): PlannedFrame[] {
  const end = videoStartMs + durationMs;
  const fixes = samples.filter(
    (s): s is Fix => s.latitude !== null && s.longitude !== null && s.timestampMs >= videoStartMs && s.timestampMs <= end,
  );
  if (fixes.length === 0) return [];
  const frame = (location: FrameLocation): PlannedFrame => ({
    videoSeconds: Math.max(0, (location.unixMs - videoStartMs) / 1000),
    location,
  });
  const frames: PlannedFrame[] = [];

  if (spacing.kind === "time") {
    const step = Math.max(spacing.seconds, 0.1) * 1000;
    let i = 0;
    for (let t = fixes[0].timestampMs; t <= fixes[fixes.length - 1].timestampMs && frames.length < maxFrames; t += step) {
      while (i < fixes.length - 2 && fixes[i + 1].timestampMs < t) i++;
      const [a, b] = [fixes[i], fixes[Math.min(i + 1, fixes.length - 1)]];
      const span = b.timestampMs - a.timestampMs;
      frames.push(frame(between(a, b, span > 0 ? (t - a.timestampMs) / span : 0)));
    }
    return frames;
  }

  // Distance: a frame at the start, then one every `meters` travelled.
  const step = Math.max(spacing.meters, 0.5);
  frames.push(frame(between(fixes[0], fixes[Math.min(1, fixes.length - 1)], 0)));
  let travelled = 0;
  let next = step;
  for (let i = 1; i < fixes.length && frames.length < maxFrames; i++) {
    const [a, b] = [fixes[i - 1], fixes[i]];
    const leg = distanceM(a.latitude, a.longitude, b.latitude, b.longitude);
    while (leg > 0 && travelled + leg >= next && frames.length < maxFrames) {
      frames.push(frame(between(a, b, (next - travelled) / leg)));
      next += step;
    }
    travelled += leg;
  }
  return frames;
}
