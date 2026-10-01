import { useEffect, useRef } from "react";
import { SphereRenderer } from "./SphereRenderer";
import { DEFAULT_VIEW, drag, zoom, type SphereView } from "./sphereView";

interface SphereViewerProps {
  /** Canvas the decoder draws each equirectangular frame on. */
  source: HTMLCanvasElement | null;
  /** Registers a callback run after every decoded frame. */
  onFrame: (listener: () => void) => () => void;
  /** Called when WebGL cannot be used, so the flat image is shown instead. */
  onUnavailable: () => void;
  /** Changing it brings the view back to straight ahead. */
  resetKey: number;
}

/** Interactive 360° view: drag to look around, pinch or scroll to zoom. */
export function SphereViewer({ source, onFrame, onUnavailable, resetKey }: SphereViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const view = useRef<SphereView>(DEFAULT_VIEW);
  const renderer = useRef<SphereRenderer | null>(null);
  const pending = useRef<number | null>(null);
  const newFrame = useRef(false);

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
    schedule.current();
  }, [resetKey]);

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
        view.current = drag(view.current, e.clientX - previous.x, e.clientY - previous.y, canvas.clientHeight);
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

  return <canvas ref={canvasRef} className="sphere-canvas" />;
}
