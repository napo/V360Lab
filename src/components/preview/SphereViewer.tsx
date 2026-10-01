import { useEffect, useRef, useState } from "react";
import { useI18n } from "../../hooks/useI18n";
import { SphereRenderer, type SphereSource } from "./SphereRenderer";
import { DEFAULT_VIEW, deviceDirection, drag, viewFromDevice, zoom, type SphereView } from "./sphereView";

interface SphereViewerProps {
  /** Element holding the equirectangular image (canvas, video or photo). */
  source: SphereSource | null;
  /** Registers a callback run whenever the source shows a new frame. */
  onFrame: (listener: () => void) => () => void;
  /** Called when WebGL cannot be used, so the flat image is shown instead. */
  onUnavailable: () => void;
  /** Changing it brings the view back to straight ahead. */
  resetKey: number;
}

type OrientationEventWithPermission = typeof DeviceOrientationEvent & {
  requestPermission?: () => Promise<"granted" | "denied">;
};

/** Phones and tablets: devices with touch and an orientation sensor API. */
function motionAvailable(): boolean {
  return typeof DeviceOrientationEvent !== "undefined" && navigator.maxTouchPoints > 0;
}

/**
 * Interactive 360° view: drag to look around, pinch or scroll to zoom, or
 * (on a phone) move the phone to look around.
 */
export function SphereViewer({ source, onFrame, onUnavailable, resetKey }: SphereViewerProps) {
  const { t } = useI18n();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const view = useRef<SphereView>(DEFAULT_VIEW);
  const renderer = useRef<SphereRenderer | null>(null);
  const pending = useRef<number | null>(null);
  const newFrame = useRef(false);
  const [motion, setMotion] = useState(false);
  // Motion control: last phone direction and the yaw it maps to.
  const device = useRef<{ heading: number; pitch: number } | null>(null);
  const yawOffset = useRef<number | null>(null);

  // Draws at most once per display refresh, only when something changed.
  const schedule = useRef(() => {});
  schedule.current = () => {
    if (pending.current !== null) return;
    pending.current = requestAnimationFrame(() => {
      pending.current = null;
      const current = renderer.current;
      if (!current) return;
      if (newFrame.current && source) {
        current.update(source);
        newFrame.current = false;
      }
      current.render(view.current);
    });
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    try {
      renderer.current = new SphereRenderer(canvas);
    } catch (e) {
      console.warn("360° view unavailable:", e);
      onUnavailable();
      return;
    }
    newFrame.current = true;
    schedule.current();
    const unsubscribe = onFrame(() => {
      newFrame.current = true;
      schedule.current();
    });
    const resize = new ResizeObserver(() => schedule.current());
    resize.observe(canvas);
    return () => {
      unsubscribe();
      resize.disconnect();
      if (pending.current !== null) cancelAnimationFrame(pending.current);
      pending.current = null;
      renderer.current?.dispose();
      renderer.current = null;
    };
  }, [source, onFrame, onUnavailable]);

  useEffect(() => {
    view.current = DEFAULT_VIEW;
    yawOffset.current = null;
    schedule.current();
  }, [resetKey]);

  // Motion control: the phone's direction drives the view.
  useEffect(() => {
    if (!motion) return;
    yawOffset.current = null;
    const listener = (e: DeviceOrientationEvent) => {
      if (e.alpha === null || e.beta === null || e.gamma === null) return;
      const direction = deviceDirection(e.alpha, e.beta, e.gamma);
      device.current = direction;
      // Start from the current view instead of jumping.
      yawOffset.current ??= view.current.yaw + direction.heading;
      view.current = viewFromDevice(view.current, direction.heading, direction.pitch, yawOffset.current);
      schedule.current();
    };
    window.addEventListener("deviceorientation", listener);
    return () => window.removeEventListener("deviceorientation", listener);
  }, [motion]);

  // Pointer handling: one pointer drags, two pointers pinch.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const pointers = new Map<number, { x: number; y: number }>();
    const distance = () => {
      const [a, b] = [...pointers.values()];
      return Math.hypot(a.x - b.x, a.y - b.y);
    };

    const down = (e: PointerEvent) => {
      canvas.setPointerCapture(e.pointerId);
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    };
    const move = (e: PointerEvent) => {
      const previous = pointers.get(e.pointerId);
      if (!previous) return;
      if (pointers.size === 1) {
        const dragged = drag(view.current, e.clientX - previous.x, e.clientY - previous.y, canvas.clientHeight);
        // With motion control, dragging only turns the view sideways.
        if (yawOffset.current !== null && device.current) {
          yawOffset.current += dragged.yaw - view.current.yaw;
          view.current = { ...view.current, yaw: dragged.yaw };
        } else {
          view.current = dragged;
        }
        pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      } else if (pointers.size === 2) {
        const before = distance();
        pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
        const after = distance();
        if (before > 0 && after > 0) view.current = zoom(view.current, before / after);
      }
      schedule.current();
    };
    const up = (e: PointerEvent) => {
      pointers.delete(e.pointerId);
    };
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      view.current = zoom(view.current, Math.exp(e.deltaY * 0.001));
      schedule.current();
    };

    canvas.addEventListener("pointerdown", down);
    canvas.addEventListener("pointermove", move);
    canvas.addEventListener("pointerup", up);
    canvas.addEventListener("pointercancel", up);
    canvas.addEventListener("wheel", wheel, { passive: false });
    return () => {
      canvas.removeEventListener("pointerdown", down);
      canvas.removeEventListener("pointermove", move);
      canvas.removeEventListener("pointerup", up);
      canvas.removeEventListener("pointercancel", up);
      canvas.removeEventListener("wheel", wheel);
    };
  }, []);

  const toggleMotion = async () => {
    if (motion) {
      setMotion(false);
      device.current = null;
      yawOffset.current = null;
      return;
    }
    // iOS asks for permission; other platforms grant it directly.
    const request = (DeviceOrientationEvent as OrientationEventWithPermission).requestPermission;
    if (request && (await request().catch(() => "denied")) !== "granted") return;
    setMotion(true);
  };

  return (
    <>
      <canvas ref={canvasRef} className="sphere-canvas" />
      {motionAvailable() && (
        <button
          type="button"
          className={`sphere-motion ${motion ? "on" : ""}`}
          aria-pressed={motion}
          title={t("preview.motion")}
          aria-label={t("preview.motion")}
          onClick={() => void toggleMotion()}
        >
          {t("preview.motionShort")}
        </button>
      )}
    </>
  );
}
