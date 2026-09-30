import { useEffect, useRef, useState } from "react";
import { Spinner } from "../Spinner";
import { ApertureIcon } from "./ApertureIcon";

export type ShutterKind = "video" | "photo" | "interval";

/** How long the diaphragm stays open for a single photo. */
const SNAP_OPEN_MS = 450;

interface ShutterButtonProps {
  kind: ShutterKind;
  /** Recording or interval capture in progress: the button stops it. */
  active: boolean;
  busy: boolean;
  disabled: boolean;
  label: string;
  onPress: () => void;
}

/**
 * The big capture button. Video: a red record button that glows while
 * recording. Photo: a closed lens diaphragm that opens when a picture is
 * taken (and stays open during an interval capture).
 */
export function ShutterButton({ kind, active, busy, disabled, label, onPress }: ShutterButtonProps) {
  const [snapping, setSnapping] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const press = () => {
    if (kind !== "video") {
      setSnapping(true);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setSnapping(false), SNAP_OPEN_MS);
    }
    onPress();
  };

  const photo = kind !== "video";
  return (
    <button
      type="button"
      className={`shutter shutter-${kind} ${active ? "active" : ""} ${busy ? "busy" : ""}`}
      disabled={disabled || busy}
      aria-label={label}
      title={label}
      onClick={press}
    >
      {photo ? (
        <>
          <ApertureIcon open={snapping || busy || active} />
          {active && <span className="shutter-stop" />}
        </>
      ) : (
        <span className="shutter-inner" />
      )}
      {busy && !photo && (
        <span className="shutter-busy">
          <Spinner size={96} />
        </span>
      )}
    </button>
  );
}
