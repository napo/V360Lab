import { describe, expect, it } from "vitest";
import { formatDurationRough, remainingMs } from "./eta";

describe("time estimates", () => {
  it("extrapolates from the items done", () => {
    expect(remainingMs(10_000, 2, 10)).toBe(40_000);
    expect(remainingMs(10_000, 0, 10)).toBeNull();
    expect(remainingMs(10_000, 10, 10)).toBe(0);
  });

  it("rounds durations for people", () => {
    expect(formatDurationRough(400)).toBe("1 s");
    expect(formatDurationRough(45_000)).toBe("45 s");
    expect(formatDurationRough(3 * 60_000)).toBe("3 min");
    expect(formatDurationRough(80 * 60_000)).toBe("80 min");
    expect(formatDurationRough(125 * 60_000)).toBe("2 h 5 min");
  });
});
