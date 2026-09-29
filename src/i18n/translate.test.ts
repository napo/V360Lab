import { describe, expect, it as test } from "vitest";
import { en } from "./en";
import { it } from "./it";
import { describeError, detectLanguage, hasKey, interpolate, translate } from "./translate";

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("dictionaries", () => {
  test("Italian has the same keys and placeholders as English", () => {
    expect(Object.keys(it).sort()).toEqual(Object.keys(en).sort());
    for (const key of Object.keys(en) as Array<keyof typeof en>) {
      expect(placeholders(it[key]), key).toEqual(placeholders(en[key]));
    }
  });

  test("no empty translations", () => {
    for (const [key, value] of Object.entries(it)) expect(value.trim(), key).not.toBe("");
  });
});

describe("translate", () => {
  test("interpolates and keeps unknown placeholders", () => {
    expect(interpolate("Hello {name} {missing}", { name: "VIRB" })).toBe("Hello VIRB {missing}");
  });

  test("selects plural forms per language", () => {
    expect(translate("en", "media.count", { count: 1, size: "1 MB" })).toBe("1 item · 1 MB");
    expect(translate("en", "media.count", { count: 3, size: "3 MB" })).toBe("3 items · 3 MB");
    expect(translate("it", "media.count", { count: 1, size: "1 MB" })).toBe("1 elemento · 1 MB");
    expect(translate("it", "media.count", { count: 0, size: "0 B" })).toBe("0 elementi · 0 B");
  });

  test("uses the plain key when a count is given but no plural forms exist", () => {
    expect(translate("en", "features.count", { count: 40 })).toBe("40 features reported");
    expect(translate("it", "features.count", { count: 40 })).toBe("40 funzioni riportate");
  });

  test("translates known keys", () => {
    expect(translate("it", "nav.settings")).toBe("Impostazioni");
    expect(translate("en", "nav.settings")).toBe("Settings");
    expect(hasKey("nav.settings")).toBe(true);
    expect(hasKey("nav.nope")).toBe(false);
  });

  test("detects the system language", () => {
    expect(detectLanguage(["it-IT", "en-US"])).toBe("it");
    expect(detectLanguage(["de-DE", "en-GB"])).toBe("en");
    expect(detectLanguage(["fr-FR"])).toBe("en");
    expect(detectLanguage([])).toBe("en");
  });
});

describe("describeError", () => {
  test("translates by kind with params", () => {
    const error = {
      kind: "timeout",
      message: "Camera did not respond within 10 s",
      detail: null,
      params: { timeoutSecs: 10 },
    };
    expect(describeError("it", error)).toBe("La camera non ha risposto entro 10 s.");
    expect(describeError("en", error)).toBe("The camera did not respond within 10 s.");
  });

  test("refines by resource or reason", () => {
    const missing = {
      kind: "missingResource",
      message: "…",
      detail: null,
      params: { name: "V0010042.MP4", resource: "fitFile" },
    };
    expect(describeError("it", missing)).toBe('"V0010042.MP4" non ha un file di telemetria FIT sulla camera.');
    const settings = { kind: "settings", message: "…", detail: null, params: { reason: "somethingNew" } };
    expect(describeError("en", settings)).toBe("Invalid settings.");
  });

  test("falls back to the backend message for unknown kinds", () => {
    const error = { kind: "internal", message: "boom", detail: null };
    expect(describeError("it", error)).toBe("boom");
  });
});
