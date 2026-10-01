import { describe, expect, it } from "vitest";
import type { AccelSample } from "../types/camera";
import {
  accelAt,
  apply,
  detectAxes,
  levelAt,
  levelingMatrix,
  tiltAngles,
  tiltSeries,
  upInImage,
  type Vec3,
} from "./level";

const reading = (t: number, x: number, y: number, z: number): AccelSample => ({ timestampMs: t, x, y, z });
const close = (a: Vec3, b: Vec3) => a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 6));

describe("horizon levelling", () => {
  it("finds the vertical axis and its direction", () => {
    expect(detectAxes([reading(0, 0.1, 0, 0.98), reading(1, -0.1, 0.05, 1.01)])?.up).toEqual({ index: 2, sign: 1 });
    expect(detectAxes([reading(0, 0, -1, 0.1)])?.up).toEqual({ index: 1, sign: -1 });
    expect(detectAxes([])).toBeNull();
  });

  it("averages readings around a time", () => {
    const samples = [reading(0, 0, 0, 1), reading(200, 0, 0, 3), reading(5000, 9, 9, 9)];
    expect(accelAt(samples, 100)).toEqual([0, 0, 2]);
    expect(accelAt(samples, 3000)).toBeNull();
  });

  it("measures roll and pitch", () => {
    const axes = { up: { index: 2, sign: 1 }, forward: { index: 0, sign: 1 } } as const;
    expect(upInImage([0, 0, 2], axes)).toEqual([0, 1, -0]);
    // Up leaning towards the camera's left (-x): the camera rolled right.
    const rolled = tiltAngles([-Math.sin(0.2), Math.cos(0.2), 0]);
    expect(rolled.rollDeg).toBeCloseTo((0.2 * 180) / Math.PI);
    expect(rolled.pitchDeg).toBeCloseTo(0);
  });

  it("maps straight up onto the camera's up and keeps the heading", () => {
    const up: Vec3 = [-Math.sin(0.3), Math.cos(0.3), 0];
    const m = levelingMatrix(up);
    close(apply(m, [0, 1, 0]), up);
    // A rotation about the forward axis leaves forward where it is.
    close(apply(m, [0, 0, -1]), [0, 0, -1]);
    expect(levelingMatrix([0, 1, 0])).toEqual([1, 0, 0, 0, 1, 0, 0, 0, 1]);
  });

  it("gives a matrix for a time with data", () => {
    const samples = [reading(0, 0, 0, 1), reading(200, 0, 0, 1)];
    const axes = detectAxes(samples);
    expect(levelAt(samples, axes, 100)).toEqual([1, 0, 0, 0, 1, 0, 0, 0, 1]);
    expect(levelAt(samples, axes, 99_999)).toBeNull();
    expect(levelAt(samples, null, 100)).toBeNull();
  });

  it("follows the tilt over time", () => {
    const tilted = (t: number, deg: number) => reading(t, Math.sin((deg * Math.PI) / 180), 0, Math.cos((deg * Math.PI) / 180));
    const samples = [tilted(0, 0), tilted(2000, 20), tilted(4000, 20)];
    const axes = { up: { index: 2, sign: 1 }, forward: { index: 0, sign: 1 } } as const;
    const series = tiltSeries(samples, axes, 500);
    expect(series).toHaveLength(3);
    expect(series[0].pitchDeg).toBeCloseTo(0);
    // +x is forward: up leaning forward means the front is pointing up.
    expect(Math.abs(series[2].pitchDeg)).toBeCloseTo(20);
    expect(series[2].rollDeg).toBeCloseTo(0);
    expect(tiltSeries(samples, null)).toEqual([]);
  });
});
