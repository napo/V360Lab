import type { CameraFeature } from "../types/camera";
import type { TranslateParams } from "./types";

type Lookup = (key: string, params?: TranslateParams) => string | null;

/** Human label for a camera feature: translation, camera label, or key. */
export function featureLabel(lookup: Lookup, feature: Pick<CameraFeature, "key" | "label">): string {
  return lookup(`feature.${feature.key}`) ?? feature.label ?? feature.key;
}

/** Human label for a feature option value; falls back to the raw value. */
export function optionLabel(lookup: Lookup, featureKey: string, value: string): string {
  return lookup(`option.${featureKey}.${value}`) ?? value;
}
