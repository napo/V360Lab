import { describe, expect, it } from "vitest";
import {
  EMPTY,
  formatBytes,
  formatDuration,
  formatJsonValue,
  formatPercent,
  storageUsedFraction,
} from "./format";
import { isAppError, toAppError } from "./errors";

describe("format helpers", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1_048_576_000)).toBe("1000 MB");
    expect(formatBytes(null)).toBe(EMPTY);
  });

  it("formats durations", () => {
    expect(formatDuration(5)).toBe("0:05");
    expect(formatDuration(125.5)).toBe("2:06");
    expect(formatDuration(3725)).toBe("1:02:05");
    expect(formatDuration(undefined)).toBe(EMPTY);
  });

  it("formats JSON values and percentages", () => {
    expect(formatJsonValue("on")).toBe("on");
    expect(formatJsonValue(3)).toBe("3");
    expect(formatJsonValue({ a: 1 })).toBe('{"a":1}');
    expect(formatJsonValue(null)).toBe(EMPTY);
    expect(formatPercent(82.4)).toBe("82%");
  });

  it("computes storage usage", () => {
    expect(storageUsedFraction(100, 25)).toBe(0.75);
    expect(storageUsedFraction(null, 25)).toBeNull();
    expect(storageUsedFraction(100, 200)).toBe(0);
  });
});

describe("error normalization", () => {
  it("keeps backend errors and wraps others", () => {
    const backend = { kind: "timeout", message: "Camera did not respond", detail: null };
    expect(isAppError(backend)).toBe(true);
    expect(toAppError(backend)).toEqual(backend);
    expect(toAppError("boom")).toEqual({ kind: "internal", message: "boom", detail: null });
    expect(toAppError(new Error("bad")).message).toBe("bad");
  });
});
