import { describe, expect, test } from "vitest";
import { parsePreviewMessage, RtpClock } from "./previewService";

const encode = (type: number, text: string) => Uint8Array.from([type, ...new TextEncoder().encode(text)]).buffer;

describe("parsePreviewMessage", () => {
  test("config", () => {
    expect(parsePreviewMessage(encode(0, '{"codec":"avc1.640028"}'))).toEqual({ type: "config", codec: "avc1.640028" });
  });

  test("frame", () => {
    const raw = Uint8Array.from([1, 1, 0x00, 0x01, 0x5f, 0x90, 0, 0, 0, 1, 0x65]);
    const message = parsePreviewMessage(raw.buffer);
    expect(message).toMatchObject({ type: "frame", keyframe: true, rtpTimestamp: 90000 });
    expect(message?.type === "frame" && Array.from(message.data)).toEqual([0, 0, 0, 1, 0x65]);
  });

  test("ended normally and with an error", () => {
    expect(parsePreviewMessage(encode(2, "null"))).toEqual({ type: "ended", error: null });
    const ended = parsePreviewMessage(
      encode(2, '{"kind":"preview","message":"x","detail":"d","params":{"reason":"stalled"}}'),
    );
    expect(ended).toMatchObject({ type: "ended", error: { kind: "preview", params: { reason: "stalled" } } });
  });

  test("rejects garbage", () => {
    expect(parsePreviewMessage(new ArrayBuffer(0))).toBeNull();
    expect(parsePreviewMessage(encode(0, "{"))).toBeNull();
    expect(parsePreviewMessage(encode(9, ""))).toBeNull();
    expect(parsePreviewMessage("text")).toBeNull();
  });
});

describe("RtpClock", () => {
  test("converts 90 kHz ticks to microseconds across wrap-around", () => {
    const clock = new RtpClock();
    expect(clock.toMicros(0xffff_ff00)).toBe(0);
    expect(clock.toMicros(0x0000_0100)).toBe(Math.floor((0x200 * 100) / 9));
    expect(clock.toMicros(0x0000_0100 + 90000)).toBe(Math.floor((0x200 * 100) / 9) + 1_000_000);
  });

  test("ignores backwards jumps", () => {
    const clock = new RtpClock();
    clock.toMicros(90000);
    expect(clock.toMicros(0)).toBe(0);
  });
});
