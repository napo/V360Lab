import { ErrorBanner } from "../components/ErrorBanner";
import { FeatureChips } from "../components/capture/FeatureChips";
import { ShutterButton, type ShutterKind } from "../components/capture/ShutterButton";
import { LivePreview } from "../components/preview/LivePreview";
import type { Navigate } from "../components/navigation";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { featureLabel } from "../i18n/featureLabels";
import type { CameraAction, CameraFeature } from "../types/camera";
import { featureValue, findFeature, findFirstFeature, optionValue } from "../utils/features";
import { connectedAccessories } from "../utils/accessories";
import { formatBytes, formatDuration, formatPercent } from "../utils/format";

const PHOTO_MODE = "photoShootingMode";

/**
 * The main capture screen, designed for a phone held in one hand: status
 * strip, video/photo switch, the settings that matter for the current mode
 * as touch chips, and a big shutter button.
 */
export function CapturePage({ navigate }: { navigate: Navigate }) {
  const camera = useCamera();
  const { t, lookup } = useI18n();
  const { features, status, pendingAction, intervalActive, runAction, connection } = camera;

  if (connection.status !== "connected") {
    return (
      <div className="page page-narrow center">
        <p className="muted">{t("capture.notConnected")}</p>
        <button type="button" className="btn btn-primary btn-hero" onClick={() => navigate("connect")}>
          {t("capture.goToConnect")}
        </button>
      </div>
    );
  }

  const state = status?.recordingState ?? "unknown";
  const recording = state === "recording";
  const photo = camera.shootingMode === PHOTO_MODE;
  const interval = photo && featureValue(findFeature(features, "photoMode")) === "Timelapse";

  const chips: Array<[CameraFeature | undefined, string]> = photo
    ? [
        [findFirstFeature(features, ["photo360Format", "video360Format"]), t("capture.lens")],
        [findFeature(features, "photoMode"), t("capture.photoType")],
        ...(interval
          ? ([
              [findFeature(features, "photoTimeLapseRate"), t("capture.interval")],
              [findFeature(features, "photoTimeLapseType"), featureLabel(lookup, { key: "photoTimeLapseType", label: null })],
            ] as Array<[CameraFeature | undefined, string]>)
          : ([[findFeature(features, "selfTimer"), t("capture.selfTimer")]] as Array<[CameraFeature | undefined, string]>)),
      ]
    : [
        [findFeature(features, "video360Format"), t("capture.lens")],
        [findFeature(features, "videoMode"), t("capture.videoType")],
      ];

  let kind: ShutterKind;
  let active: boolean;
  let action: CameraAction;
  let label: string;
  if (recording || !photo) {
    // A recording started elsewhere is always stoppable from here.
    kind = "video";
    active = recording;
    action = recording ? "stopRecording" : "startRecording";
    label = recording ? t("capture.shutterStop") : t("capture.shutterRecord");
  } else if (interval) {
    kind = "interval";
    active = intervalActive;
    action = intervalActive ? "stopStillRecording" : "snapPicture";
    label = intervalActive ? t("capture.shutterIntervalStop") : t("capture.shutterIntervalStart");
  } else {
    kind = "photo";
    active = false;
    action = "snapPicture";
    label = t("capture.shutterPhoto");
  }

  // Settings whose change makes the camera restart its preview stream.
  const previewKey = ["shootingMode", "video360Format", "photo360Format", "videoMode", "photoMode"]
    .map((key) => featureValue(findFeature(features, key)) ?? "")
    .join("|");
  const modeFeature = findFeature(features, "shootingMode");
  const settingsLocked = recording || camera.pendingFeature !== null;

  return (
    <div className="page capture-page">
      <div className="capture-status">
        <span className={`rec-indicator ${recording || intervalActive ? "on" : ""}`}>
          <span className="rec-dot" />
          {recording
            ? formatDuration(status?.recordingTimeSecs)
            : intervalActive
              ? t("capture.intervalRunning")
              : t(`recording.${state}`)}
        </span>
        {status?.batteryLevel != null && (
          <span>{t("capture.battery", { level: formatPercent(status.batteryLevel) })}</span>
        )}
        {status?.storageAvailableBytes != null && (
          <span>{t("capture.free", { free: formatBytes(status.storageAvailableBytes) })}</span>
        )}
        {connectedAccessories(status).map((key) => (
          <span key={key} className="badge accessory-badge">
            {t(key)}
          </span>
        ))}
      </div>

      <LivePreview restartKey={previewKey} spherical={featureValue(chips[0][0]) === "360"} />

      {modeFeature && (
        <div className="segmented segmented-large" role="group" aria-label={t("capture.mode")}>
          {modeFeature.options.map(optionValue).map((option) => (
            <button
              key={option}
              type="button"
              className={option === camera.shootingMode ? "active" : undefined}
              disabled={settingsLocked || intervalActive || option === camera.shootingMode}
              onClick={() => void camera.updateFeature(modeFeature.key, option)}
            >
              {lookup(`option.shootingMode.${option}`) ?? option}
            </button>
          ))}
        </div>
      )}

      <div className="chip-fields">
        {chips.map(([feature, chipLabel]) =>
          feature ? <FeatureChips key={feature.key} feature={feature} label={chipLabel} /> : null,
        )}
      </div>

      <div className="shutter-area">
        <ShutterButton
          kind={kind}
          active={active}
          busy={pendingAction !== null}
          disabled={camera.pendingFeature !== null}
          label={label}
          onPress={() => void runAction(action)}
        />
        <span className="shutter-label">{label}</span>
      </div>

      {camera.featureError && <ErrorBanner error={camera.featureError} title={t("capture.settingFailed")} />}
      {camera.actionError && <ErrorBanner error={camera.actionError} title={t("actions.failed")} />}
      {camera.statusError && <ErrorBanner error={camera.statusError} title={t("status.unavailable")} />}
    </div>
  );
}
