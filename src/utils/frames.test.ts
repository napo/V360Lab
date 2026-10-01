import { describe, expect, it } from "vitest";
import type { TelemetrySample } from "../types/camera";
import { bearingDeg, distanceM, planFrames } from "./frames";

const fix = (t: number, lat: number, lon: number): TelemetrySample => ({
  timestampMs: t,
  latitude: lat,
  longitude: lon,
  altitudeM: 200,
  speedMps: 5,
  headingDeg: null,
  heartRate: null,
});

// Northwards, ~11.1 m per second.
const track = Array.from({ length: 11 }, (_, i) => fix(1000 + i * 1000, 46 + i * 0.0001, 11));

describe("frame planning", () => {
  it("measures distance and bearing", () => {
    expect(distanceM(46, 11, 46.001, 11)).toBeCloseTo(111.2, 0);
    expect(bearingDeg(46, 11, 46.001, 11)).toBeCloseTo(0, 3);
    expect(bearingDeg(46, 11, 46, 11.001)).toBeCloseTo(90, 0);
  });

  it("places frames every N metres, interpolating between fixes", () => {
    const frames = planFrames(track, 0, 20_000, { kind: "distance", meters: 25 });
    // ~111 m travelled: frames at 0, 25, 50, 75, 100 m.
    expect(frames).toHaveLength(5);
    expect(frames[0].videoSeconds).toBeCloseTo(1);
    expect(distanceM(46, 11, frames[1].location.latitude, frames[1].location.longitude)).toBeCloseTo(25, 0);
    expect(frames[1].location.headingDeg).toBeCloseTo(0, 3);
    expect(frames[2].videoSeconds).toBeGreaterThan(frames[1].videoSeconds);
  });

  it("places frames every N seconds within the video", () => {
    const frames = planFrames(track, 3000, 4000, { kind: "time", seconds: 2 });
    expect(frames.map((f) => f.videoSeconds)).toEqual([0, 2, 4]);
    expect(frames[1].location.latitude).toBeCloseTo(46.0004, 6);
  });

  it("returns nothing without positions and respects the limit", () => {
    expect(planFrames([{ ...fix(0, 0, 0), latitude: null, longitude: null }], 0, 1000, { kind: "time", seconds: 1 })).toEqual([]);
    expect(planFrames(track, 0, 20_000, { kind: "distance", meters: 1 }, 10)).toHaveLength(10);
  });
});
