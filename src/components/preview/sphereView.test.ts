import { describe, expect, it } from "vitest";
import {
  DEFAULT_VIEW,
  MAX_FOV,
  MIN_FOV,
  deviceDirection,
  drag,
  isEquirectangular,
  viewFromDevice,
  zoom,
} from "./sphereView";

describe("sphere view", () => {
  it("recognizes equirectangular frames", () => {
    expect(isEquirectangular(1920, 960)).toBe(true);
    // The VIRB's live preview: the whole sphere squeezed into 16:9.
    expect(isEquirectangular(640, 368)).toBe(true);
    expect(isEquirectangular(1280, 720)).toBe(true);
    expect(isEquirectangular(1440, 1080)).toBe(false);
    expect(isEquirectangular(1000, 1000)).toBe(false);
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

  it("turns the phone's orientation into a direction", () => {
    // Upright in portrait, facing the reference direction: straight ahead.
    const upright = deviceDirection(0, 90, 0);
    expect(upright.heading).toBeCloseTo(0);
    expect(upright.pitch).toBeCloseTo(0);
    // Tilted back: looks up.
    expect(deviceDirection(0, 120, 0).pitch).toBeCloseTo(Math.PI / 6);
    // Turned left by 90° (alpha grows counter-clockwise): heading -90°.
    expect(deviceDirection(90, 90, 0).heading).toBeCloseTo(-Math.PI / 2);
  });

  it("keeps the starting view when motion control starts", () => {
    const start = { ...DEFAULT_VIEW, yaw: 0.5 };
    const offset = start.yaw + 0.2;
    expect(viewFromDevice(start, 0.2, 0, offset).yaw).toBeCloseTo(0.5);
    // Turning right (heading grows) looks right (yaw decreases).
    expect(viewFromDevice(start, 0.7, 0, offset).yaw).toBeCloseTo(0);
  });
});
