import type { SphereView } from "./sphereView";

const VERTEX_SHADER = `
attribute vec2 position;
varying vec2 screen;
void main() {
  screen = position;
  gl_Position = vec4(position, 0.0, 1.0);
}`;

// For each pixel, the direction the virtual camera looks at, turned into
// longitude/latitude and looked up in the equirectangular image.
const FRAGMENT_SHADER = `
precision highp float;
uniform sampler2D image;
uniform float yaw;
uniform float pitch;
uniform float fov;
uniform float aspect;
varying vec2 screen;
const float PI = 3.141592653589793;
void main() {
  float t = tan(fov * 0.5);
  vec3 dir = normalize(vec3(screen.x * t * aspect, screen.y * t, -1.0));
  float cp = cos(pitch);
  float sp = sin(pitch);
  dir = vec3(dir.x, dir.y * cp - dir.z * sp, dir.y * sp + dir.z * cp);
  float cy = cos(yaw);
  float sy = sin(yaw);
  dir = vec3(dir.x * cy + dir.z * sy, dir.y, -dir.x * sy + dir.z * cy);
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
  private readonly uniforms: Record<"yaw" | "pitch" | "fov" | "aspect", WebGLUniformLocation | null>;
  private readonly texture: WebGLTexture | null;

  /** Throws when WebGL is unavailable. */
  constructor(private readonly canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl", { antialias: false, alpha: false });
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
    };

    // Video sizes are not powers of two: no mipmaps, clamped edges.
    this.texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  /** Uploads the latest decoded frame. */
  update(source: HTMLCanvasElement) {
    if (source.width === 0 || source.height === 0) return;
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, gl.RGB, gl.UNSIGNED_BYTE, source);
  }

  render(view: SphereView) {
    const gl = this.gl;
    const canvas = this.canvas;
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
