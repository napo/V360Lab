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

/**
 * Whether an image from a 360° lens can be an equirectangular panorama
 * (360° × 180°). Files are 2:1, but the VIRB squeezes the live preview into
 * 16:9 (e.g. 640 × 360): the shader maps the whole image onto the sphere
 * whatever its proportions. Square or 4:3 images are not panoramas.
 */
export function isEquirectangular(width: number, height: number): boolean {
  if (width <= 0 || height <= 0) return false;
  const ratio = width / height;
  return ratio >= 1.5 && ratio <= 2.3;
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

/**
 * Where the back of the phone points, from a `deviceorientation` event
 * (degrees, W3C Z-X'-Y'' angles): compass heading (radians, clockwise from
 * the reference direction) and pitch (radians, positive up). The phone can
 * be held in portrait or landscape: only the direction of its back counts.
 */
export function deviceDirection(alpha: number, beta: number, gamma: number): { heading: number; pitch: number } {
  const [a, b, g] = [alpha * DEG, beta * DEG, gamma * DEG];
  const [sa, ca, sb, cb, sg, cg] = [Math.sin(a), Math.cos(a), Math.sin(b), Math.cos(b), Math.sin(g), Math.cos(g)];
  // -Z axis of the device (out of its back) in east/north/up coordinates.
  const x = -sg * ca - cg * sb * sa;
  const y = -sg * sa + cg * sb * ca;
  const z = -cg * cb;
  return { heading: Math.atan2(x, y), pitch: Math.asin(Math.max(-1, Math.min(1, z))) };
}

/** View for a phone pointing at `heading`/`pitch`; `yawOffset` keeps the
 * view where it was when motion control was turned on. */
export function viewFromDevice(view: SphereView, heading: number, pitch: number, yawOffset: number): SphereView {
  return {
    ...view,
    yaw: wrapAngle(yawOffset - heading),
    pitch: Math.min(MAX_PITCH, Math.max(-MAX_PITCH, pitch)),
  };
}
