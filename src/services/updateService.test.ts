import { describe, expect, it } from "vitest";
import { releaseHighlights } from "./updateService";

describe("release highlights", () => {
  it("keeps the bullet points of the What's new section, without Markdown", () => {
    const notes = [
      "## What's new",
      "",
      "- **Download the model**: from the app.",
      "- Uses `models-v1`.",
      "",
      "## Downloads",
      "- Windows installer",
    ].join("\n");
    expect(releaseHighlights(notes)).toEqual(["Download the model: from the app.", "Uses models-v1."]);
  });

  it("works without sections and limits the length", () => {
    expect(releaseHighlights("- a\n- b\n- c", 2)).toEqual(["a", "b"]);
    expect(releaseHighlights("")).toEqual([]);
  });
});
