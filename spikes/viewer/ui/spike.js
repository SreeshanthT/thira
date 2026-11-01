// Throwaway measurement page for issue #2.
const { invoke } = window.__TAURI__.core;
const status = document.getElementById("status");

function stats(values) {
  const v = [...values].sort((a, b) => a - b);
  const pick = (q) => v[Math.round((v.length - 1) * q)];
  const mean = v.reduce((a, b) => a + b, 0) / v.length;
  return { frames: v.length, median_ms: pick(0.5), p95_ms: pick(0.95), max_ms: pick(1), fps_at_mean: 1000 / mean };
}

function setupGl(canvas) {
  const gl = canvas.getContext("webgl2", { antialias: false });
  const compile = (type, src) => {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    return s;
  };
  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, `#version 300 es
    out vec2 uv;
    void main() {
      vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
      uv = vec2(p.x * 0.5, 1.0 - p.y * 0.5);
      gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    }`));
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, `#version 300 es
    precision mediump float;
    in vec2 uv; uniform sampler2D tex; out vec4 color;
    void main() { color = texture(tex, uv); }`));
  gl.linkProgram(prog);
  gl.useProgram(prog);
  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return gl;
}

async function runTransfer(frames) {
  const canvas = document.createElement("canvas");
  document.body.appendChild(canvas);
  canvas.width = canvas.clientWidth * devicePixelRatio;
  canvas.height = canvas.clientHeight * devicePixelRatio;
  const gl = setupGl(canvas);
  const results = {};
  for (const [width, height] of [[1920, 1080], [960, 540]]) {
    const total = [], rust = [], ipc = [], upload = [];
    for (let i = -10; i < frames; i++) {
      const t0 = performance.now();
      const buf = await invoke("frame", { width, height, t: t0 / 1000 });
      const t1 = performance.now();
      const rustMs = new DataView(buf, 0, 8).getFloat64(0, true) / 1000;
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(buf, 8));
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.finish();
      const t2 = performance.now();
      if (i < 0) continue; // warm-up
      total.push(t2 - t0);
      rust.push(rustMs);
      ipc.push(t1 - t0 - rustMs);
      upload.push(t2 - t1);
      if (i % 30 === 0) status.textContent = `${width}x${height}: frame ${i}/${frames}`;
    }
    results[`${width}x${height}`] = {
      total: stats(total),
      render_and_readback: stats(rust),
      ipc_transfer: stats(ipc),
      webgl_upload_and_draw: stats(upload),
      mb_per_frame: (width * height * 4) / 1e6,
    };
  }
  results.user_agent = navigator.userAgent;
  await invoke("report", { results });
}

function runNative() {
  document.body.className = "native";
  const menu = document.createElement("div");
  menu.className = "menu";
  menu.textContent = "HTML menu over the native viewer: it must stay on top and be readable.";
  document.body.appendChild(menu);
  status.textContent = "native surface: rendering under the transparent webview";
}

const cfg = await invoke("config");
if (cfg.mode === "native") {
  runNative();
} else {
  document.body.className = "transfer";
  runTransfer(cfg.frames).catch((e) => invoke("report", { results: { error: String(e) } }));
}
