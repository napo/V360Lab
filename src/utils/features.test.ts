import { describe, expect, it } from "vitest";
import type { CameraFeature } from "../types/camera";
import { formatNumberLike, isEditable, isNumber } from "./features";

function feature(value: string, options: string[] = [], featureType = 1): CameraFeature {
  return { key: "k", label: null, value, options, optionSummaries: [], enabled: true, featureType, raw: null };
}

describe("numeric features", () => {
  it("recognises a free numeric value", () => {
    expect(isNumber(feature("9.000000"))).toBe(true);
    expect(isEditable(feature("9.000000"))).toBe(true);
  });

  it("leaves text, choices and toggles out", () => {
    expect(isNumber(feature("VIRB360"))).toBe(false);
    expect(isNumber(feature("2m", ["2m", "5m"]))).toBe(false);
    expect(isNumber(feature("1", [], 2))).toBe(false);
    expect(isNumber(feature(""))).toBe(false);
  });

  it("writes the value as the camera does", () => {
    expect(formatNumberLike("9.000000", 1)).toBe("1.000000");
    expect(formatNumberLike("9", 2)).toBe("2");
    expect(formatNumberLike(null, 3)).toBe("3");
  });
});
