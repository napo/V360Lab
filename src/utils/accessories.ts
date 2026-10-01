import type { CameraStatus } from "../types/camera";
import type { TranslationKey } from "../i18n/types";

/** Accessories connected to the camera, as reported by its status. */
export function connectedAccessories(status: CameraStatus | null): TranslationKey[] {
  if (!status) return [];
  const accessories: TranslationKey[] = [];
  if (status.bluetoothHeadset) accessories.push("accessory.headset");
  if (status.bluetoothSensor) accessories.push("accessory.bluetoothSensor");
  if (status.antSensor) accessories.push("accessory.antSensor");
  return accessories;
}
