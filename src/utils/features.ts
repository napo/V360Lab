import type { CameraFeature, FeatureList } from "../types/camera";

/** VIRB feature types: 0 = action, 1 = choice/value, 2 = on/off toggle. */
export const FEATURE_TYPE_ACTION = 0;
export const FEATURE_TYPE_TOGGLE = 2;

export function findFeature(list: FeatureList | null, key: string): CameraFeature | undefined {
  return list?.features.find((f) => f.key === key);
}

/** First feature present among `keys` (names differ between modes/firmware). */
export function findFirstFeature(list: FeatureList | null, keys: string[]): CameraFeature | undefined {
  for (const key of keys) {
    const feature = findFeature(list, key);
    if (feature) return feature;
  }
  return undefined;
}

/** Feature value as the string the camera expects in `updateFeature`. */
export function featureValue(feature: CameraFeature | undefined): string | null {
  const value = feature?.value;
  if (value === null || value === undefined) return null;
  return typeof value === "string" ? value : JSON.stringify(value);
}

export function optionValue(option: unknown): string {
  return typeof option === "string" ? option : JSON.stringify(option);
}

export function isToggle(feature: CameraFeature): boolean {
  return feature.featureType === FEATURE_TYPE_TOGGLE;
}

export function isChoice(feature: CameraFeature): boolean {
  return feature.options.length > 0 && feature.featureType !== FEATURE_TYPE_ACTION;
}

/** A free numeric value, such as `wifiTimeout` ("9.000000"). */
export function isNumber(feature: CameraFeature): boolean {
  if (feature.featureType !== 1 || feature.options.length > 0) return false;
  const value = featureValue(feature);
  return value !== null && value.trim() !== "" && Number.isFinite(Number(value));
}

/** `value` written like the camera writes `current` ("9.000000" → "1.000000"). */
export function formatNumberLike(current: string | null, value: number): string {
  const decimals = current?.split(".")[1]?.length ?? 0;
  return value.toFixed(decimals);
}

/** Features V360Lab can change from the UI. */
export function isEditable(feature: CameraFeature): boolean {
  return feature.enabled !== false && (isToggle(feature) || isChoice(feature) || isNumber(feature));
}
