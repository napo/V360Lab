import { describe, expect, test } from "vitest";
import { optionLabel } from "./featureLabels";
import { hasKey, translate } from "./translate";
import type { TranslateParams } from "./types";

describe("optionLabel", () => {
  const it = (key: string, params?: TranslateParams) => (hasKey(key) ? translate("it", key, params) : null);

  test("adds the unit to numeric self-timer values", () => {
    expect(optionLabel(it, "selfTimer", "10")).toBe("10 s");
    expect(optionLabel(it, "selfTimer", "Off")).toBe("Disattivato");
  });

  test("leaves values of features without a unit unchanged", () => {
    expect(optionLabel(it, "photoTimeLapseRate", "5s")).toBe("5s");
  });
});
