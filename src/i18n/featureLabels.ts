import type { CameraFeature } from "../types/camera";
import type { TranslateParams } from "./types";

type Lookup = (key: string, params?: TranslateParams) => string | null;

/** Human label for a camera feature: translation, camera label, or key. */
export function featureLabel(lookup: Lookup, feature: Pick<CameraFeature, "key" | "label">): string {
  return lookup(`feature.${feature.key}`) ?? feature.label ?? feature.key;
}

/**
 * Human label for a feature option value; falls back to the raw value.
 * Plain numbers get the feature's unit when one is defined (`unit.<key>`),
 * e.g. the self-timer's "10" becomes "10 s".
 */
export function optionLabel(lookup: Lookup, featureKey: string, value: string): string {
  const translated = lookup(`option.${featureKey}.${value}`);
  if (translated !== null) return translated;
  if (/^\d+$/.test(value)) return lookup(`unit.${featureKey}`, { value }) ?? value;
  return value;
}
