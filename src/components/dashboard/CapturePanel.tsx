import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { featureLabel, optionLabel } from "../../i18n/featureLabels";
import { featureValue, findFeature, findFirstFeature, optionValue } from "../../utils/features";
import { ErrorBanner } from "../ErrorBanner";
import { FeatureControl } from "../features/FeatureControl";
import { Panel } from "../Panel";

const PHOTO_MODE = "photoShootingMode";

/**
 * Capture controls: video/photo mode, lens format and the settings relevant
 * to the current mode, plus start/stop buttons.
 */
export function CapturePanel() {
  const camera = useCamera();
  const { features, status, pendingAction, pendingFeature, shootingMode, runAction } = camera;
  const { t, lookup } = useI18n();

  const state = status?.recordingState ?? "unknown";
  const recording = state === "recording";
  const busy = pendingAction !== null;
  const settingsLocked = recording || pendingFeature !== null;

  const modeFeature = findFeature(features, "shootingMode");
  const photo = shootingMode === PHOTO_MODE;
  const intervalCapture = photo && featureValue(findFeature(features, "photoMode")) === "Timelapse";

  // Settings shown for the current mode, in this order; missing ones are skipped.
  const fieldKeys: string[][] = photo
    ? [
        ["photo360Format", "video360Format"],
        ["photoMode"],
        ...(intervalCapture ? [["photoTimeLapseRate"], ["photoTimeLapseType"]] : [["selfTimer"]]),
      ]
    : [["video360Format"], ["videoMode"], ["videoLoop"]];
  const fields = fieldKeys
    .map((keys) => findFirstFeature(features, keys))
    .filter((feature) => feature !== undefined);

  return (
    <Panel title={t("capture.panel")}>
      {modeFeature && (
        <div className="segmented" role="group" aria-label={t("capture.mode")}>
          {modeFeature.options.map(optionValue).map((option) => (
            <button
              key={option}
              type="button"
              className={option === shootingMode ? "active" : undefined}
              disabled={settingsLocked || option === shootingMode}
              onClick={() => void camera.updateFeature(modeFeature.key, option)}
            >
              {optionLabel(lookup, modeFeature.key, option)}
            </button>
          ))}
        </div>
      )}

      {fields.length > 0 && (
        <div className="capture-fields">
          {fields.map((feature) => (
            <label key={feature.key} className="capture-field">
              <span>{featureLabel(lookup, feature)}</span>
              <FeatureControl feature={feature} />
            </label>
          ))}
        </div>
      )}
      {recording && <p className="muted small">{t("capture.lockedWhileRecording")}</p>}
      {intervalCapture && <p className="muted small">{t("capture.intervalHint")}</p>}

      <div className="action-grid">
        {!photo && (
          <>
            <button
              type="button"
              className="btn btn-record"
              disabled={busy || recording}
              onClick={() => void runAction("startRecording")}
            >
              {pendingAction === "startRecording" ? t("actions.starting") : t("actions.start")}
            </button>
            <button
              type="button"
              className="btn"
              disabled={busy || state === "idle"}
              onClick={() => void runAction("stopRecording")}
            >
              {pendingAction === "stopRecording" ? t("actions.stopping") : t("actions.stop")}
            </button>
          </>
        )}
        {photo && intervalCapture && (
          <>
            <button
              type="button"
              className="btn btn-record"
              disabled={busy || recording}
              onClick={() => void runAction("snapPicture")}
            >
              {pendingAction === "snapPicture" ? t("actions.starting") : t("capture.startInterval")}
            </button>
            <button
              type="button"
              className="btn"
              disabled={busy}
              onClick={() => void runAction("stopStillRecording")}
            >
              {pendingAction === "stopStillRecording" ? t("actions.stopping") : t("capture.stopInterval")}
            </button>
          </>
        )}
        {photo && !intervalCapture && (
          <button
            type="button"
            className="btn btn-record"
            disabled={busy || recording}
            onClick={() => void runAction("snapPicture")}
          >
            {pendingAction === "snapPicture" ? t("actions.capturing") : t("actions.photo")}
          </button>
        )}
        {/* A recording started elsewhere can always be stopped from photo mode too. */}
        {photo && recording && (
          <button type="button" className="btn" disabled={busy} onClick={() => void runAction("stopRecording")}>
            {t("actions.stop")}
          </button>
        )}
        <button
          type="button"
          className="btn btn-ghost"
          disabled={camera.refreshing}
          onClick={() => void camera.refreshAll()}
        >
          {t("actions.refresh")}
        </button>
      </div>

      {state === "unknown" && <p className="muted small">{t("actions.unknownState")}</p>}
      {camera.featuresError && !features && (
        <ErrorBanner
          error={camera.featuresError}
          title={t("capture.featuresUnavailable")}
          onRetry={() => void camera.loadFeatures()}
        />
      )}
      {camera.featureError && <ErrorBanner error={camera.featureError} title={t("capture.settingFailed")} />}
      {camera.actionError && <ErrorBanner error={camera.actionError} title={t("actions.failed")} />}
    </Panel>
  );
}
