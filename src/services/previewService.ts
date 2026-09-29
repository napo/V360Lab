import { Channel } from "@tauri-apps/api/core";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";
import { call } from "./backend";

/** Messages of the live preview channel (see `src-tauri/src/preview`). */
export type PreviewMessage =
  | { type: "config"; codec: string }
  | { type: "frame"; keyframe: boolean; rtpTimestamp: number; data: Uint8Array }
  | { type: "ended"; error: AppError | null };

const MSG_CONFIG = 0;
const MSG_FRAME = 1;
const MSG_ENDED = 2;
const decoder = new TextDecoder();

function toBytes(data: unknown): Uint8Array | null {
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (data instanceof Uint8Array) return data;
  if (Array.isArray(data)) return Uint8Array.from(data as number[]);
  return null;
}

/** Parses one binary message; returns null for anything unexpected. */
export function parsePreviewMessage(raw: unknown): PreviewMessage | null {
  const bytes = toBytes(raw);
  if (!bytes || bytes.length === 0) return null;
  const payload = bytes.subarray(1);
  try {
    switch (bytes[0]) {
      case MSG_CONFIG: {
        const { codec } = JSON.parse(decoder.decode(payload)) as { codec?: unknown };
        return typeof codec === "string" ? { type: "config", codec } : null;
      }
      case MSG_FRAME: {
        if (bytes.length < 6) return null;
        const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        return {
          type: "frame",
          keyframe: bytes[1] === 1,
          rtpTimestamp: view.getUint32(2),
          data: bytes.subarray(6),
        };
      }
      case MSG_ENDED: {
        const error = JSON.parse(decoder.decode(payload)) as unknown;
        return { type: "ended", error: error === null ? null : toAppError(error) };
      }
    }
  } catch {
    return null;
  }
  return null;
}

/**
 * Turns 32-bit, 90 kHz RTP timestamps into monotonic microseconds for
 * WebCodecs, surviving wrap-around.
 */
export class RtpClock {
  private last: number | null = null;
  private ticks = 0;

  toMicros(rtpTimestamp: number): number {
    if (this.last !== null) {
      const delta = (rtpTimestamp - this.last) >>> 0;
      // Backwards jumps (reordering, stream restart) do not move the clock.
      if (delta < 0x8000_0000) this.ticks += delta;
    }
    this.last = rtpTimestamp;
    return Math.floor((this.ticks * 100) / 9);
  }
}

export const previewService = {
  /** Starts the preview; resolves once the camera is streaming. */
  start: (onMessage: (message: PreviewMessage) => void) => {
    const channel = new Channel<unknown>();
    channel.onmessage = (raw) => {
      const message = parsePreviewMessage(raw);
      if (message) onMessage(message);
    };
    return call<void>("start_preview", { channel });
  },
  stop: () => call<void>("stop_preview"),
};
