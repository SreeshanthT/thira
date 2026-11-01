import { useEffect, useRef, useState } from "react";
import { FrameCanvas } from "./frameCanvas";
import { setViewerRect, viewerBackend, viewerFrame, type ViewerBackend } from "./ipc";

/** Physical-pixel rectangle of an element, from the top-left of the webview. */
function physicalRect(el: Element) {
  const r = el.getBoundingClientRect();
  const d = window.devicePixelRatio || 1;
  return {
    x: Math.max(0, Math.round(r.left * d)),
    y: Math.max(0, Math.round(r.top * d)),
    width: Math.round(r.width * d),
    height: Math.round(r.height * d),
  };
}

/**
 * The program viewer (ADR 0003). With the native backend this element is a
 * transparent hole: the shell draws video under the webview at the rectangle we
 * report. With the transfer backend it pulls frames and draws them on a canvas.
 */
export function Viewer() {
  const hole = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [backend, setBackend] = useState<ViewerBackend | null>(null);

  useEffect(() => {
    viewerBackend().then(setBackend, () => setBackend(null));
  }, []);

  // Lets the stylesheet make the page transparent only for the native backend.
  useEffect(() => {
    if (backend) document.documentElement.dataset.viewer = backend;
  }, [backend]);

  // Native: keep the shell informed of where the hole is.
  useEffect(() => {
    const el = hole.current;
    if (backend !== "native" || !el) return;
    const report = () => void setViewerRect(physicalRect(el)).catch(() => undefined);
    report();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(report);
    observer?.observe(el);
    window.addEventListener("resize", report);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", report);
      void setViewerRect(null).catch(() => undefined);
    };
  }, [backend]);

  // Transfer: request frames one at a time and draw them.
  useEffect(() => {
    const el = canvas.current;
    if (backend !== "transfer" || !el) return;
    let stopped = false;
    let painter: FrameCanvas;
    try {
      painter = new FrameCanvas(el);
    } catch {
      return;
    }
    const tick = async () => {
      if (stopped) return;
      const { width, height } = physicalRect(el);
      if (width > 0 && height > 0) {
        if (el.width !== width || el.height !== height) {
          el.width = width;
          el.height = height;
        }
        try {
          painter.draw(await viewerFrame(width, height), width, height);
        } catch {
          // Keep the last frame; try again next tick.
        }
      }
      requestAnimationFrame(() => void tick());
    };
    void tick();
    return () => {
      stopped = true;
    };
  }, [backend]);

  return (
    <div className="viewer" ref={hole} data-backend={backend ?? undefined}>
      {backend === "transfer" && <canvas ref={canvas} />}
    </div>
  );
}
