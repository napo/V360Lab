import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { optionLabel } from "../../i18n/featureLabels";
import type { CameraFeature } from "../../types/camera";
import { featureValue, isChoice, isEditable, isToggle, optionValue } from "../../utils/features";
import { formatJsonValue } from "../../utils/format";

interface FeatureControlProps {
  feature: CameraFeature;
  disabled?: boolean;
}

/**
 * Edits one camera feature: a select for choices, a checkbox for on/off
 * toggles, plain text otherwise. Changes are sent to the camera at once.
 * Locked while recording or while another change is being applied.
 */
export function FeatureControl({ feature, disabled = false }: FeatureControlProps) {
  const { updateFeature, pendingFeature, status } = useCamera();
  const { t, lookup } = useI18n();
  const recording = status?.recordingState === "recording";
  const locked = disabled || recording || pendingFeature !== null || !isEditable(feature);
  const current = featureValue(feature);
  const applying = pendingFeature === feature.key && (
    <span className="muted small">{t("capture.applying")}</span>
  );

  if (isToggle(feature)) {
    const on = current === "1";
    return (
      <label className="checkbox">
        <input
          type="checkbox"
          checked={on}
          disabled={locked}
          onChange={(e) => void updateFeature(feature.key, e.target.checked ? "1" : "0")}
        />
        {on ? t("common.on") : t("common.off")}
        {applying}
      </label>
    );
  }

  if (isChoice(feature)) {
    const options = feature.options.map(optionValue);
    return (
      <span className="feature-control">
        <select
          value={current ?? ""}
          disabled={locked}
          onChange={(e) => void updateFeature(feature.key, e.target.value)}
        >
          {/* Keep an unexpected current value visible instead of hiding it. */}
          {current !== null && !options.includes(current) && <option value={current}>{current}</option>}
          {options.map((option) => (
            <option key={option} value={option}>
              {optionLabel(lookup, feature.key, option)}
            </option>
          ))}
        </select>
        {applying}
      </span>
    );
  }

  return <span className="mono">{formatJsonValue(feature.value)}</span>;
}
