import { RtpClock, type PreviewMessage } from "../../services/previewService";

/**
 * Frames waiting in the decoder above which we skip to the next keyframe.
 * Hardware decoders hold a few frames, and 360° frames take longer: a
 * lower limit made the 360° preview jump from keyframe to keyframe.
 */
const MAX_DECODE_QUEUE = 16;

export interface PlayerEvents {
  onFirstFrame: (width: number, height: number) => void;
  /** After every frame drawn on the canvas. */
  onFrame?: () => void;
  onDecodeError: (message: string) => void;
}

export function webCodecsAvailable(): boolean {
  return typeof VideoDecoder !== "undefined" && typeof EncodedVideoChunk !== "undefined";
}

/**
 * Decodes the preview's H.264 access units with WebCodecs and draws each
 * frame on a canvas. Favors latency: when decoding falls behind, frames are
 * dropped until the next keyframe.
 */
export class PreviewPlayer {
  private decoder: VideoDecoder | null = null;
  /** Last codec announced by the backend (sent only when it changes). */
  private codec: string | null = null;
  private waitingForKeyframe = true;
  private clock = new RtpClock();
  private frames = 0;
  private closed = false;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly events: PlayerEvents,
  ) {}

  handle(message: PreviewMessage) {
    if (this.closed) return;
    if (message.type === "config") this.configure(message.codec);
    else if (message.type === "frame") this.decode(message);
  }

  close() {
    this.closed = true;
    this.resetDecoder();
  }

  private configure(codec: string) {
    if (codec === this.codec && this.decoder?.state === "configured") return;
    this.resetDecoder();
    this.codec = codec;
    this.createDecoder(codec);
  }

  private createDecoder(codec: string) {
    this.decoder = new VideoDecoder({
      output: (frame) => this.draw(frame),
      error: (e) => {
        this.events.onDecodeError(e.message);
        // Recreated on the next keyframe.
        this.decoder = null;
      },
    });
    // No `description`: chunks are Annex B with in-band SPS/PPS.
    this.decoder.configure({ codec, optimizeForLatency: true });
    this.waitingForKeyframe = true;
  }

  private decode(message: Extract<PreviewMessage, { type: "frame" }>) {
    if ((!this.decoder || this.decoder.state === "closed") && message.keyframe && this.codec) {
      this.createDecoder(this.codec);
    }
    const decoder = this.decoder;
    if (!decoder || decoder.state !== "configured") return;
    if (!message.keyframe && (this.waitingForKeyframe || decoder.decodeQueueSize > MAX_DECODE_QUEUE)) {
      this.waitingForKeyframe = true;
      return;
    }
    this.waitingForKeyframe = false;
    decoder.decode(
      new EncodedVideoChunk({
        type: message.keyframe ? "key" : "delta",
        timestamp: this.clock.toMicros(message.rtpTimestamp),
        data: message.data,
      }),
    );
  }

  private draw(frame: VideoFrame) {
    try {
      if (this.closed) return;
      const { displayWidth: width, displayHeight: height } = frame;
      if (this.canvas.width !== width || this.canvas.height !== height) {
        this.canvas.width = width;
        this.canvas.height = height;
      }
      this.canvas.getContext("2d")?.drawImage(frame, 0, 0, width, height);
      if (this.frames++ === 0) this.events.onFirstFrame(width, height);
      this.events.onFrame?.();
    } finally {
      frame.close();
    }
  }

  private resetDecoder() {
    if (this.decoder && this.decoder.state !== "closed") this.decoder.close();
    this.decoder = null;
    this.codec = null;
  }
}
