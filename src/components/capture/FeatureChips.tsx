import { useCamera } from "../../hooks/useCamera";
import { useI18n } from "../../hooks/useI18n";
import { optionLabel } from "../../i18n/featureLabels";
import type { CameraFeature } from "../../types/camera";
import { featureValue, optionValue } from "../../utils/features";
import { FeatureControl } from "../features/FeatureControl";

/** Above this many options a compact select is easier than chips. */
const MAX_CHIPS = 7;

interface FeatureChipsProps {
  feature: CameraFeature;
  label: string;
}

/**
 * A camera feature as a row of large touch targets: one chip per option,
 * the current one highlighted. Tapping applies the value immediately.
 */
export function FeatureChips({ feature, label }: FeatureChipsProps) {
  const { updateFeature, pendingFeature, status } = useCamera();
  const { lookup } = useI18n();
  const options = feature.options.map(optionValue);
  const current = featureValue(feature);
  const locked =
    status?.recordingState === "recording" || pendingFeature !== null || feature.enabled === false;

  return (
    <div className="chip-field">
      <span className="chip-label">{label}</span>
      {options.length > MAX_CHIPS ? (
        <FeatureControl feature={feature} />
      ) : (
        <div className="chips" role="radiogroup" aria-label={label}>
          {options.map((option) => (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={option === current}
              className={`chip ${option === current ? "active" : ""} ${pendingFeature === feature.key ? "pending" : ""}`}
              disabled={locked || option === current}
              onClick={() => void updateFeature(feature.key, option)}
            >
              {lookup(`chip.${feature.key}.${option}`) ?? optionLabel(lookup, feature.key, option)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
