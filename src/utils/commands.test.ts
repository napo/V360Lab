import { describe, expect, it } from "vitest";
import { commandSupported } from "./commands";

describe("commandSupported", () => {
  it("trusts a list that names the basic commands", () => {
    expect(commandSupported(["status", "locate"], "locate")).toBe(true);
    expect(commandSupported(["status", "locate"], "networks")).toBe(false);
  });

  it("assumes support when the list is missing or unexpected", () => {
    expect(commandSupported(null, "networks")).toBe(true);
    expect(commandSupported([], "networks")).toBe(true);
    expect(commandSupported(["getStatus"], "networks")).toBe(true);
  });
});
