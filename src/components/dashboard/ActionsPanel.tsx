import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { ErrorBanner } from "../ErrorBanner";
import { Panel } from "../Panel";

export function ActionsPanel() {
  const { status, pendingAction, actionError, refreshing, runAction, refreshAll } = useCamera();
  const { t } = useI18n();
  const state = status?.recordingState ?? "unknown";
  const busy = pendingAction !== null;

  // With an unknown state both start and stop stay available, since the
  // camera itself will reject an incompatible command.
  const canStart = !busy && state !== "recording";
  const canStop = !busy && state !== "idle";
  const canSnap = !busy && state !== "recording";

  return (
    <Panel title={t("actions.panel")}>
      <div className="action-grid">
        <button
          type="button"
          className="btn btn-record"
          disabled={!canStart}
          onClick={() => void runAction("startRecording")}
        >
          {pendingAction === "startRecording" ? t("actions.starting") : t("actions.start")}
        </button>
        <button
          type="button"
          className="btn"
          disabled={!canStop}
          onClick={() => void runAction("stopRecording")}
        >
          {pendingAction === "stopRecording" ? t("actions.stopping") : t("actions.stop")}
        </button>
        <button
          type="button"
          className="btn"
          disabled={!canSnap}
          onClick={() => void runAction("snapPicture")}
        >
          {pendingAction === "snapPicture" ? t("actions.capturing") : t("actions.photo")}
        </button>
        <button type="button" className="btn btn-ghost" disabled={refreshing} onClick={() => void refreshAll()}>
          {t("actions.refresh")}
        </button>
      </div>
      {state === "unknown" && <p className="muted small">{t("actions.unknownState")}</p>}
      {actionError && <ErrorBanner error={actionError} title={t("actions.failed")} />}
    </Panel>
  );
}
