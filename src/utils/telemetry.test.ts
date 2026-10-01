import { describe, expect, it } from "vitest";
import type { TelemetrySample } from "../types/camera";
import { nearestPoint, projectTrack, sampleIndexAt, seriesPath } from "./telemetry";

const sample = (t: number, lat: number | null, lon: number | null, speed: number | null = null): TelemetrySample => ({
  timestampMs: t,
  latitude: lat,
  longitude: lon,
  altitudeM: null,
  speedMps: speed,
  headingDeg: null,
  heartRate: null,
});

describe("telemetry helpers", () => {
  const samples = [sample(0, 46, 11, 1), sample(1000, 46.001, 11, 2), sample(2000, 46.002, 11.001, 3)];

  it("finds the sample at a time", () => {
    expect(sampleIndexAt(samples, -5)).toBe(0);
    expect(sampleIndexAt(samples, 400)).toBe(0);
    expect(sampleIndexAt(samples, 600)).toBe(1);
    expect(sampleIndexAt(samples, 9999)).toBe(2);
    expect(sampleIndexAt([], 0)).toBe(-1);
  });

  it("projects north up inside the box", () => {
    const points = projectTrack(samples, 100, 100);
    expect(points[0]!.y).toBeGreaterThan(points[2]!.y);
    for (const p of points) {
      expect(p!.x).toBeGreaterThanOrEqual(0);
      expect(p!.x).toBeLessThanOrEqual(100);
    }
    expect(nearestPoint(points, points[1]!.x, points[1]!.y)).toBe(1);
    expect(projectTrack([sample(0, null, null)], 10, 10)).toEqual([null]);
  });

  it("draws a series over the video time", () => {
    const path = seriesPath(samples, (s) => s.speedMps, 0, 2000, 200, 50);
    expect(path.split(" ")).toHaveLength(3);
    expect(path.startsWith("0.0,48.0")).toBe(true);
  });
});
