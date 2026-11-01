/** Draws RGBA frames on a canvas with WebGL2 (transfer backend, ADR 0003). */
export class FrameCanvas {
  private readonly gl: WebGL2RenderingContext;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2", { antialias: false, alpha: false });
    if (!gl) throw new Error("WebGL2 is not available");
    this.gl = gl;

    const compile = (type: number, source: string) => {
      const shader = gl.createShader(type);
      if (!shader) throw new Error("cannot create shader");
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      return shader;
    };
    const program = gl.createProgram();
    // A full-screen triangle; uv maps the texture's first row to the top.
    gl.attachShader(
      program,
      compile(
        gl.VERTEX_SHADER,
        `#version 300 es
        out vec2 uv;
        void main() {
          vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
          uv = vec2(p.x * 0.5, 1.0 - p.y * 0.5);
          gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
        }`,
      ),
    );
    gl.attachShader(
      program,
      compile(
        gl.FRAGMENT_SHADER,
        `#version 300 es
        precision mediump float;
        in vec2 uv;
        uniform sampler2D frame;
        out vec4 color;
        void main() { color = texture(frame, uv); }`,
      ),
    );
    gl.linkProgram(program);
    gl.useProgram(program);
    gl.bindTexture(gl.TEXTURE_2D, gl.createTexture());
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  /** Uploads a tightly packed RGBA frame and draws it over the whole canvas. */
  draw(pixels: ArrayBuffer, width: number, height: number) {
    const gl = this.gl;
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      width,
      height,
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      new Uint8Array(pixels),
    );
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}
