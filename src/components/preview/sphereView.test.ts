import { describe, expect, it } from "vitest";
import { DEFAULT_VIEW, MAX_FOV, MIN_FOV, drag, isEquirectangular, zoom } from "./sphereView";

describe("sphere view", () => {
  it("recognizes equirectangular frames", () => {
    expect(isEquirectangular(1920, 960)).toBe(true);
    expect(isEquirectangular(1280, 720)).toBe(false);
    expect(isEquirectangular(0, 0)).toBe(false);
  });

  it("follows the finger and keeps the view upright", () => {
    const right = drag(DEFAULT_VIEW, 100, 0, 500);
    expect(right.yaw).toBeGreaterThan(0);
    expect(right.pitch).toBe(0);
    const down = drag(DEFAULT_VIEW, 0, 10_000, 500);
    expect(down.pitch).toBeLessThan(Math.PI / 2);
    expect(down.pitch).toBeGreaterThan(1.5);
  });

  it("wraps around horizontally", () => {
    let view = DEFAULT_VIEW;
    for (let i = 0; i < 50; i++) view = drag(view, 200, 0, 300);
    expect(Math.abs(view.yaw)).toBeLessThanOrEqual(Math.PI);
  });

  it("limits the zoom", () => {
    expect(zoom(DEFAULT_VIEW, 100).fov).toBe(MAX_FOV);
    expect(zoom(DEFAULT_VIEW, 0.01).fov).toBe(MIN_FOV);
    expect(zoom(DEFAULT_VIEW, Number.NaN)).toBe(DEFAULT_VIEW);
  });
});
