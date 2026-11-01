# Viewer spike (issue #2)

Throwaway prototype behind [ADR 0003](../../docs/adr/0003-viewer-presentation.md). It is not part of the main workspace and is not shipped.

It renders an animated test pattern with wgpu and measures two ways of putting it in the Tauri window:

| Mode | What it does |
| --- | --- |
| `transfer` (default) | Renders offscreen, reads the frame back, sends it to the webview as binary IPC and draws it with WebGL. Measures 1920×1080 and 960×540. |
| `native` | Presents straight to the window under a transparent webview, with and without vsync. An HTML box is drawn over the video to check it stays on top. |

```sh
cargo build --release
THIRA_SPIKE_MODE=native THIRA_SPIKE_FRAMES=300 THIRA_SPIKE_HOLD_MS=10000 ./target/release/thira-viewer-spike
```

Results are printed and written to `THIRA_SPIKE_OUT` (default `spike-result.json`). The `Viewer spike` workflow runs both modes on all three OSes and uploads results and screenshots.
