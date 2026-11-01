# 0003. Viewer presentation: native surface under a transparent webview

| | |
| --- | --- |
| Status | Accepted |
| Date | 2026-11-01 |
| Issue | #2 |

## Context

The viewer shows frames rendered by wgpu (`thira-render`) inside the Tauri window, whose UI is a webview. PRD G1 and the playback requirements need 1080p at 30 fps with no dropped frames and scrubbing under 100 ms, with the same behavior on Windows, macOS and Linux. ADR 0001 left open how frames reach the screen.

Two options were prototyped in [`spikes/viewer`](../../spikes/viewer) and measured with the same animated test pattern:

- **A. Native surface.** wgpu presents directly to the Tauri window. The webview is transparent, so the video shows through it wherever the UI leaves a hole; HTML (toolbars, menus) draws on top.
- **B. Frame transfer.** wgpu renders offscreen, the frame is read back to the CPU and sent to the webview as binary IPC (`tauri::ipc::Response`), where WebGL draws it to a `<canvas>`.

## Measurements

Median milliseconds per frame. For B, the total is the full round trip: render and read back, IPC, then WebGL upload and draw. CI runners have no real GPU (software or paravirtual adapters), so CI numbers are indicative only.

### B: frame transfer

| Machine | Adapter | 1920×1080 total | of which IPC | 960×540 total |
| --- | --- | --- | --- | --- |
| Windows 11, local | Intel UHD (Vulkan) | **118** (≈8 fps) | 106 | 50 |
| Windows, CI | Microsoft Basic Render (DX12/WARP) | 109 | 93 | 34 |
| macOS, CI | Apple Paravirtual (Metal) | **15** | 5 | 7 |
| Linux, CI (Xvfb) | llvmpipe (Vulkan) | 31 | 21 | 13 |

GPU render and readback was never the bottleneck (8–17 ms at 1080p). Moving 8.3 MB through Tauri IPC into WebView2 runs at roughly 80 MB/s, so B cannot reach 30 fps at 1080p on Windows. On macOS (WKWebView) it is fast.

### A: native surface

| Machine | Adapter | Vsync (FIFO) | No vsync (Immediate) | HTML stays on top |
| --- | --- | --- | --- | --- |
| Windows 11, local | Intel UHD (Vulkan) | 16.6 (60 fps) | **1.2** | Yes (screenshot) |
| Windows, CI | WARP (DX12) | 15.8 | 5.1–8.3 | Yes (screenshot) |
| macOS, CI | Apple Paravirtual (Metal) | 16.7 | 3.0 | Yes (screenshot) |
| Linux, CI (Xvfb) | llvmpipe (Vulkan) | 2.5 (no vsync under Xvfb) | 2.5 | **Unverified**: screen capture under Xvfb returned black for both modes |

## Decision

Use **A, the native surface**, on every OS, behind a small `ViewerBackend` interface in the engine, with **B kept as a fallback backend**.

- Windows and macOS use the native surface.
- Linux uses the native surface once it is confirmed on real hardware with a compositor ([#74](https://github.com/SreeshanthT/thira/issues/74)). Until then Linux defaults to frame transfer, which measured fast enough there, and the native surface can be enabled by setting.
- Either backend can be forced by a setting, as a workaround for driver problems.

### How the native surface works

- The webview is transparent (`transparent: true`). The UI paints opaque panels everywhere except the viewer areas, which are transparent holes.
- The UI reports each viewer's rectangle (`getBoundingClientRect()` × `devicePixelRatio`) to the engine when the layout changes. The renderer draws each viewer (source and program) into its rectangle on the one window surface, using viewport and scissor; the rest is cleared to the UI background color.
- The surface is reconfigured on window resize and scale-factor changes.

### Constraints found by the spike

- **macOS:** the surface must be created on the main thread (creating it elsewhere panics in `raw-window-metal`); rendering and presenting can then run on the render thread.
- **macOS:** transparent windows need Tauri's `macos-private-api` feature, which rules out the Mac App Store. Thira ships as a signed DMG (PRD section 9), so this is acceptable.
- Rendering uses `Fifo` (vsync) for playback, so the viewer never tears; the frame budget at 60 Hz is 16.7 ms.

## Alternatives considered

- **B everywhere.** One code path, but 8 fps at 1080p on Windows. Making it fast would need WebView2 shared buffers through `unsafe` Windows APIs, or compressing every frame, both with uncertain results. Kept only as a fallback.
- **A separate native child window over the viewer area.** Avoids transparency, but HTML menus and popups could never draw over the video, and window positioning differs per OS. Not prototyped.

## Consequences

- Playback cost is GPU rendering only; no per-frame copies through the webview.
- The UI layout must keep the engine informed of viewer rectangles, and the window background is the renderer's job in viewer areas.
- Two backends exist, so the viewer interface must stay small; golden-frame tests (#12) render offscreen and cover both.
- Linux needs a confirmation run on real hardware (#74) before the native surface becomes its default.
