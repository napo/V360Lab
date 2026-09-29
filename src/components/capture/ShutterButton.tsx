import { Spinner } from "../Spinner";

export type ShutterKind = "video" | "photo" | "interval";

interface ShutterButtonProps {
  kind: ShutterKind;
  /** Recording or interval capture in progress: the button stops it. */
  active: boolean;
  busy: boolean;
  disabled: boolean;
  label: string;
  onPress: () => void;
}

/** The big round capture button, styled like a camera shutter. */
export function ShutterButton({ kind, active, busy, disabled, label, onPress }: ShutterButtonProps) {
  return (
    <button
      type="button"
      className={`shutter shutter-${kind} ${active ? "active" : ""}`}
      disabled={disabled || busy}
      aria-label={label}
      title={label}
      onClick={onPress}
    >
      <span className="shutter-inner" />
      {busy && (
        <span className="shutter-busy">
          <Spinner size={96} />
        </span>
      )}
    </button>
  );
}
