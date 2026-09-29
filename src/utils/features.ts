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

/** Features V360Lab can change from the UI. */
export function isEditable(feature: CameraFeature): boolean {
  return feature.enabled !== false && (isToggle(feature) || isChoice(feature));
}
