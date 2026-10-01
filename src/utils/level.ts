import type { AccelSample } from "../types/camera";

/**
 * Horizon levelling of 360° images from the camera's accelerometer.
 *
 * Image frame (as in the 360° shader): x right, y up, z backwards (the
 * centre of the image looks along -z). The accelerometer measures about
 * +1 g pointing up when the camera is still, so averaged over a second its
 * direction is "up" as seen by the camera.
 *
 * The camera's sensor axes are mapped to the image frame by `CameraAxes`.
 * The vertical axis is found from the data (the camera is mostly upright);
 * which of the other two points forward is a property of the VIRB 360 that
 * still has to be confirmed on a real camera: see `FORWARD_AXIS_GUESS`.
 */

export type Vec3 = [number, number, number];
/** Row-major 3×3 matrix. */
export type Mat3 = [number, number, number, number, number, number, number, number, number];

export interface AxisRef {
  index: 0 | 1 | 2;
  sign: 1 | -1;
}

export interface CameraAxes {
  up: AxisRef;
  forward: AxisRef;
}

export const IDENTITY: Mat3 = [1, 0, 0, 0, 1, 0, 0, 0, 1];

/**
 * Sensor axis pointing out of the front lens (the centre of the 360°
 * image), for each vertical axis. HYPOTHESIS, to be checked on a VIRB 360.
 */
const FORWARD_AXIS_GUESS: Record<0 | 1 | 2, AxisRef> = {
  0: { index: 1, sign: 1 },
  1: { index: 0, sign: 1 },
  2: { index: 0, sign: 1 },
};

const component = (a: AccelSample, i: 0 | 1 | 2) => (i === 0 ? a.x : i === 1 ? a.y : a.z);

/** Vertical axis: the one carrying most of gravity on average. */
export function detectAxes(samples: AccelSample[]): CameraAxes | null {
  if (samples.length === 0) return null;
  const mean = ([0, 1, 2] as const).map((i) => samples.reduce((sum, a) => sum + component(a, i), 0) / samples.length);
  const index = ([0, 1, 2] as const).reduce((best, i) => (Math.abs(mean[i]) > Math.abs(mean[best]) ? i : best), 0);
  if (Math.abs(mean[index]) < 1e-6) return null;
  return { up: { index, sign: mean[index] > 0 ? 1 : -1 }, forward: FORWARD_AXIS_GUESS[index] };
}

/** Average reading within ±windowMs/2 of `timeMs` (samples sorted by time). */
export function accelAt(samples: AccelSample[], timeMs: number, windowMs = 1000): Vec3 | null {
  let low = 0;
  let high = samples.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (samples[mid].timestampMs < timeMs - windowMs / 2) low = mid + 1;
    else high = mid;
  }
  const sum: Vec3 = [0, 0, 0];
  let count = 0;
  for (let i = low; i < samples.length && samples[i].timestampMs <= timeMs + windowMs / 2; i++) {
    sum[0] += samples[i].x;
    sum[1] += samples[i].y;
    sum[2] += samples[i].z;
    count++;
  }
  return count > 0 ? [sum[0] / count, sum[1] / count, sum[2] / count] : null;
}

const unit = (axis: AxisRef): Vec3 => {
  const v: Vec3 = [0, 0, 0];
  v[axis.index] = axis.sign;
  return v;
};
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const normalize = (v: Vec3): Vec3 | null => {
  const length = Math.hypot(...v);
  return length > 1e-9 ? [v[0] / length, v[1] / length, v[2] / length] : null;
};

/** "Up" seen by the camera, in the image frame. */
export function upInImage(reading: Vec3, axes: CameraAxes): Vec3 | null {
  const up = unit(axes.up);
  const forward = unit(axes.forward);
  const right = cross(forward, up);
  return normalize([dot(reading, right), dot(reading, up), -dot(reading, forward)]);
}

/** Camera tilt in degrees: roll (positive = tilted to the right) and pitch
 * (positive = front lens pointing down). */
export function tiltAngles(up: Vec3): { rollDeg: number; pitchDeg: number } {
  const deg = 180 / Math.PI;
  return { rollDeg: Math.atan2(-up[0], up[1]) * deg, pitchDeg: Math.atan2(up[2], up[1]) * deg };
}

/**
 * Matrix taking a direction of the levelled image to the direction to read
 * in the camera's image: it maps straight up (0, 1, 0) onto the camera's
 * `up`, by the smallest rotation (so the view keeps its heading).
 */
export function levelingMatrix(up: Vec3): Mat3 {
  const target: Vec3 = [0, 1, 0];
  const axis = cross(target, up);
  const sin = Math.hypot(...axis);
  const cos = dot(target, up);
  if (sin < 1e-9) return cos > 0 ? IDENTITY : [1, 0, 0, 0, -1, 0, 0, 0, -1];
  const [x, y, z] = [axis[0] / sin, axis[1] / sin, axis[2] / sin];
  const t = 1 - cos;
  // Rodrigues' rotation formula.
  return [
    t * x * x + cos, t * x * y - sin * z, t * x * z + sin * y,
    t * x * y + sin * z, t * y * y + cos, t * y * z - sin * x,
    t * x * z - sin * y, t * y * z + sin * x, t * z * z + cos,
  ];
}

export function apply(m: Mat3, v: Vec3): Vec3 {
  return [
    m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
    m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
    m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
  ];
}

/** Levelling matrix at `timeMs`, or null without usable data. */
export function levelAt(samples: AccelSample[], axes: CameraAxes | null, timeMs: number): Mat3 | null {
  if (!axes) return null;
  const reading = accelAt(samples, timeMs);
  const up = reading && upInImage(reading, axes);
  return up ? levelingMatrix(up) : null;
}
