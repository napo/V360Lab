import { useCamera } from "../../hooks/useCamera";
import { ErrorBanner } from "../ErrorBanner";
import { Panel } from "../Panel";

export function ActionsPanel() {
  const { status, pendingAction, actionError, refreshing, runAction, refreshAll } = useCamera();
  const state = status?.recordingState ?? "unknown";
  const busy = pendingAction !== null;

  // With an unknown state both start and stop stay available, since the
  // camera itself will reject an incompatible command.
  const canStart = !busy && state !== "recording";
  const canStop = !busy && state !== "idle";
  const canSnap = !busy && state !== "recording";

  return (
    <Panel title="Actions">
      <div className="action-grid">
        <button
          type="button"
          className="btn btn-record"
          disabled={!canStart}
          onClick={() => void runAction("startRecording")}
        >
          {pendingAction === "startRecording" ? "Starting…" : "Start recording"}
        </button>
        <button
          type="button"
          className="btn"
          disabled={!canStop}
          onClick={() => void runAction("stopRecording")}
        >
          {pendingAction === "stopRecording" ? "Stopping…" : "Stop recording"}
        </button>
        <button
          type="button"
          className="btn"
          disabled={!canSnap}
          onClick={() => void runAction("snapPicture")}
        >
          {pendingAction === "snapPicture" ? "Capturing…" : "Take photo"}
        </button>
        <button type="button" className="btn btn-ghost" disabled={refreshing} onClick={() => void refreshAll()}>
          Refresh status
        </button>
      </div>
      {state === "unknown" && (
        <p className="muted small">Recording state unknown: controls are not restricted.</p>
      )}
      {actionError && <ErrorBanner error={actionError} title="Command failed" />}
    </Panel>
  );
}
