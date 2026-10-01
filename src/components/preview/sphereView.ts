/** Where the 360° viewer looks, in radians. */
export interface SphereView {
  /** Horizontal angle; 0 = straight ahead (centre of the image). */
  yaw: number;
  /** Vertical angle; positive looks up. */
  pitch: number;
  /** Vertical field of view. */
  fov: number;
}

const DEG = Math.PI / 180;
export const MIN_FOV = 30 * DEG;
export const MAX_FOV = 110 * DEG;
const MAX_PITCH = 89 * DEG;

export const DEFAULT_VIEW: SphereView = { yaw: 0, pitch: 0, fov: 80 * DEG };

/** Equirectangular images cover 360° × 180°, so they are about 2:1. */
export function isEquirectangular(width: number, height: number): boolean {
  if (width <= 0 || height <= 0) return false;
  const ratio = width / height;
  return ratio > 1.8 && ratio < 2.2;
}

function wrapAngle(angle: number): number {
  const turn = 2 * Math.PI;
  return ((((angle + Math.PI) % turn) + turn) % turn) - Math.PI;
}

/**
 * Drags the view by `dx`, `dy` CSS pixels on a viewer `height` pixels tall:
 * the image follows the finger, as in a photo sphere viewer.
 */
export function drag(view: SphereView, dx: number, dy: number, height: number): SphereView {
  const radiansPerPixel = view.fov / Math.max(height, 1);
  return {
    ...view,
    yaw: wrapAngle(view.yaw + dx * radiansPerPixel),
    pitch: Math.min(MAX_PITCH, Math.max(-MAX_PITCH, view.pitch + dy * radiansPerPixel)),
  };
}

/** Zooms by `factor` (>1 zooms out), within the allowed field of view. */
export function zoom(view: SphereView, factor: number): SphereView {
  if (!Number.isFinite(factor) || factor <= 0) return view;
  return { ...view, fov: Math.min(MAX_FOV, Math.max(MIN_FOV, view.fov * factor)) };
}
