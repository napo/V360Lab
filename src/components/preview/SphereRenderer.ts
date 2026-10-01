import type { Mat3 } from "../../utils/level";
import type { SphereView } from "./sphereView";

/** Anything showing an equirectangular image: the live preview's canvas,
 * a recorded video or a photo. */
export type SphereSource = HTMLCanvasElement | HTMLVideoElement | HTMLImageElement;

export function sourceSize(source: SphereSource): { width: number; height: number } {
  if (source instanceof HTMLVideoElement) return { width: source.videoWidth, height: source.videoHeight };
  if (source instanceof HTMLImageElement) return { width: source.naturalWidth, height: source.naturalHeight };
  return { width: source.width, height: source.height };
}

const VERTEX_SHADER = `
attribute vec2 position;
varying vec2 screen;
void main() {
  screen = position;
  gl_Position = vec4(position, 0.0, 1.0);
}`;

// For each pixel, the direction the virtual camera looks at (perspective
// view, or every direction for an equirectangular output), corrected by
// the levelling matrix, turned into longitude/latitude and looked up in
// the equirectangular image.
const FRAGMENT_SHADER = `
precision highp float;
uniform sampler2D image;
uniform float yaw;
uniform float pitch;
uniform float fov;
uniform float aspect;
uniform float equirectangular;
uniform mat3 level;
varying vec2 screen;
const float PI = 3.141592653589793;
void main() {
  vec3 dir;
  if (equirectangular > 0.5) {
    float outLon = screen.x * PI;
    float outLat = screen.y * PI * 0.5;
    dir = vec3(cos(outLat) * sin(outLon), sin(outLat), -cos(outLat) * cos(outLon));
  } else {
    float t = tan(fov * 0.5);
    dir = normalize(vec3(screen.x * t * aspect, screen.y * t, -1.0));
    float cp = cos(pitch);
    float sp = sin(pitch);
    dir = vec3(dir.x, dir.y * cp - dir.z * sp, dir.y * sp + dir.z * cp);
    float cy = cos(yaw);
    float sy = sin(yaw);
    dir = vec3(dir.x * cy + dir.z * sy, dir.y, -dir.x * sy + dir.z * cy);
  }
  dir = level * dir;
  float lon = atan(dir.x, -dir.z);
  float lat = asin(clamp(dir.y, -1.0, 1.0));
  gl_FragColor = texture2D(image, vec2(lon / (2.0 * PI) + 0.5, 0.5 - lat / PI));
}`;

function compile(gl: WebGLRenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("WebGL: cannot create shader");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(`WebGL: ${gl.getShaderInfoLog(shader) ?? "shader error"}`);
  }
  return shader;
}

/**
 * Shows a perspective view of an equirectangular (360° × 180°) image, like
 * Garmin's app does with the VIRB 360 preview. The image comes from a
 * canvas the video decoder draws on.
 */
export class SphereRenderer {
  private readonly gl: WebGLRenderingContext;
  private readonly uniforms: Record<
    "yaw" | "pitch" | "fov" | "aspect" | "equirectangular" | "level",
    WebGLUniformLocation | null
  >;
  private readonly texture: WebGLTexture | null;

  /** Largest texture side the GPU accepts. */
  readonly maxTextureSize: number;

  /** Throws when WebGL is unavailable. `preserveDrawingBuffer` keeps the
   * image after drawing, to read it back (frame extraction). */
  constructor(
    private readonly canvas: HTMLCanvasElement,
    preserveDrawingBuffer = false,
  ) {
    const gl = canvas.getContext("webgl", { antialias: false, alpha: false, preserveDrawingBuffer });
    if (!gl) throw new Error("WebGL is not available");
    this.gl = gl;

    const program = gl.createProgram();
    if (!program) throw new Error("WebGL: cannot create program");
    gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER));
    gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      throw new Error(`WebGL: ${gl.getProgramInfoLog(program) ?? "link error"}`);
    }
    gl.useProgram(program);

    // One triangle pair covering the whole canvas.
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

    this.uniforms = {
      yaw: gl.getUniformLocation(program, "yaw"),
      pitch: gl.getUniformLocation(program, "pitch"),
      fov: gl.getUniformLocation(program, "fov"),
      aspect: gl.getUniformLocation(program, "aspect"),
      equirectangular: gl.getUniformLocation(program, "equirectangular"),
      level: gl.getUniformLocation(program, "level"),
    };
    this.maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;

    // Video sizes are not powers of two: no mipmaps, clamped edges.
    this.texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  /** Uploads the current frame of the source. */
  update(source: SphereSource) {
    const { width, height } = sourceSize(source);
    if (width === 0 || height === 0) return;
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    try {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, gl.RGB, gl.UNSIGNED_BYTE, source);
    } catch (e) {
      // e.g. a cross-origin image the webview refuses to use as a texture.
      console.warn("360° view: cannot read the image", e);
    }
  }

  /** WebGL matrices are column-major. */
  private setLevel(level: Mat3 | null) {
    const m = level ?? [1, 0, 0, 0, 1, 0, 0, 0, 1];
    this.gl.uniformMatrix3fv(this.uniforms.level, false, [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]);
  }

  /** Draws the whole sphere as an equirectangular image of the canvas's
   * size, levelled by `level`. */
  renderEquirectangular(level: Mat3 | null) {
    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.uniform1f(this.uniforms.equirectangular, 1);
    this.setLevel(level);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  render(view: SphereView, level: Mat3 | null = null) {
    const gl = this.gl;
    const canvas = this.canvas;
    gl.uniform1f(this.uniforms.equirectangular, 0);
    this.setLevel(level);
    // Match the displayed size for a sharp image.
    const scale = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(canvas.clientWidth * scale));
    const height = Math.max(1, Math.round(canvas.clientHeight * scale));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    gl.uniform1f(this.uniforms.yaw, view.yaw);
    gl.uniform1f(this.uniforms.pitch, view.pitch);
    gl.uniform1f(this.uniforms.fov, view.fov);
    gl.uniform1f(this.uniforms.aspect, width / height);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  dispose() {
    this.gl.deleteTexture(this.texture);
  }
}
