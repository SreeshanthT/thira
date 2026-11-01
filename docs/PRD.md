# Thira: Product Requirements Document

| | |
| --- | --- |
| Status | Draft v0.1 |
| Date | 2026-11-01 |
| Owner | @SreeshanthT |
| Project | Open source, Apache-2.0 |
| Platforms | Windows, macOS, Linux |

## 1. Summary

Thira is an open-source, cross-platform, non-destructive video editor focused on **editing**: assembling, trimming, timing, and exporting video quickly and reliably. It runs on Windows, macOS and Linux with the same features and behavior on each.

The first public release (v0.1) is a usable editor for single-person projects. Color grading, audio mixing, compositing and AI features are out of scope until the editor is solid.

## 2. Problem

- Professional editors are closed source, tied to one vendor's subscription or hardware, or limited on Linux.
- Existing open-source editors are often unstable on long projects, slow on large timelines, or lack a clean cross-platform release story.
- Editors who want to learn, extend, or script their tools have no modern, well-documented codebase to build on.

## 3. Vision and goals

**Vision:** a fast, dependable, open editing core that other tools can build on.

| # | Goal | How we measure it |
| --- | --- | --- |
| G1 | Editing feels instant | Scrubbing and timeline operations respond in under 100 ms on the reference machine (section 8) |
| G2 | Never lose work | Zero data-loss bugs; crash recovery restores the project to within 30 seconds of the crash |
| G3 | Same on every OS | One feature set and one test suite passing on Windows, macOS and Linux |
| G4 | Open and extensible | Public API and plugin points documented before v1.0 |
| G5 | Sustainable for one maintainer plus AI assistance | Small, well-tested modules; CI green on every merge |

## 4. Non-goals (for v1.0)

- Color grading beyond basic corrections, node-based compositing, 3D, and particle effects.
- Multi-track audio mixing and plugin hosting (basic audio editing only).
- AI features (transcription, auto-edit, masking).
- Multi-user collaboration and cloud services.
- Mobile and web versions.
- Matching the interface or branding of any commercial editor.

## 5. Users

| Persona | Needs | Priority |
| --- | --- | --- |
| **Solo creator** (YouTube, social video) | Fast cuts, titles, captions, quick export presets | Primary |
| **Hobbyist / student** | Free, simple, cross-platform, no watermark | Primary |
| **Linux editor** | A modern native-feeling editor with hardware acceleration | Primary |
| **Open-source contributor** | Clean architecture, good docs, easy local build | Secondary |
| **Scripter / tool builder** | Automate repetitive edits via an API | Secondary (post v1.0) |

## 6. Product principles

1. **Non-destructive:** source media is never modified.
2. **Responsive first:** if a feature slows editing, it ships behind a proxy or cache.
3. **Predictable:** the same project produces the same export on every OS.
4. **Recoverable:** every edit is undoable; autosave is always on.
5. **Documented:** every feature has user docs and tests before release.

## 7. Functional requirements

Priority: **P0** = required for v0.1, **P1** = required for v1.0, **P2** = after v1.0.

### 7.1 Projects

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| PRJ-1 | Create, open, save and close projects | P0 | Reopened project is identical to the saved one |
| PRJ-2 | Autosave and crash recovery | P0 | After a forced kill, reopening offers recovery of the last autosave |
| PRJ-3 | Project settings: resolution, frame rate, aspect ratio | P0 | Settings apply to new timelines; changing them re-renders without data loss |
| PRJ-4 | Relink missing media | P1 | Moved folder can be relinked in one step for all missing files |
| PRJ-5 | Project file format is documented and versioned | P1 | Older projects open in newer versions via migrations |

### 7.2 Media management

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| MED-1 | Import video, audio and images by dialog and drag-and-drop | P0 | Common files (MP4, MOV, MKV, WebM, MP3, WAV, PNG, JPG) import without conversion |
| MED-2 | Media pool with bins, search and thumbnails | P0 | 1,000 clips browse without lag |
| MED-3 | Metadata display (codec, resolution, frame rate, duration) | P0 | Matches `ffprobe` output |
| MED-4 | Background waveform and thumbnail generation | P0 | UI stays interactive during generation |
| MED-5 | Proxy generation and automatic proxy playback | P1 | 4K source plays smoothly on the reference machine using proxies |
| MED-6 | Image sequence import | P1 | Numbered frames import as one clip |

### 7.3 Timeline editing

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| TL-1 | Multi-track video and audio timeline | P0 | At least 8 video and 8 audio tracks |
| TL-2 | Insert, overwrite, append, delete, ripple delete | P0 | Each operation matches expected clip positions in automated tests |
| TL-3 | Trim tools: ripple, roll, slip, slide | P0 | Edit points update on both sides correctly; unit tests cover edge cases |
| TL-4 | Blade/split at playhead; join clips | P0 | Splitting then joining restores the original |
| TL-5 | Snapping, markers, in/out points | P0 | Snaps to clip edges, markers and playhead |
| TL-6 | Undo and redo with unlimited depth in a session | P0 | 1,000 operations undo and redo exactly |
| TL-7 | Speed change, reverse, freeze frame | P1 | Audio pitch option; frame-accurate |
| TL-8 | Nested (compound) clips | P1 | Edit inside, changes reflect on the timeline |
| TL-9 | Multicam sync and switching | P1 | Sync by timecode or audio waveform |
| TL-10 | Keyframes with easing for position, scale, rotation, opacity, volume | P1 | Bezier easing editable in a curve view |
| TL-11 | Adjustment layers | P2 | Effects apply to all tracks below |

### 7.4 Playback and viewer

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| PLY-1 | Play, pause, step frame, jog, shuttle (J/K/L) | P0 | Frame-accurate stepping |
| PLY-2 | Audio/video sync | P0 | Drift under 1 frame over 10 minutes |
| PLY-3 | Scrubbing without stalls | P0 | See G1 |
| PLY-4 | Full-screen and second-monitor viewer | P1 | Opens on any display |
| PLY-5 | Safe-area guides and aspect-ratio overlays | P2 | Toggleable overlays |

### 7.5 Effects, titles and transitions

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| FX-1 | Basic transform: position, scale, rotation, crop | P0 | Applies in viewer and export identically |
| FX-2 | Cross dissolve, dip to color, wipe | P0 | Adjustable duration and alignment |
| FX-3 | Text titles with font, size, color, outline, shadow | P1 | Fonts from the system render the same on all OSes |
| FX-4 | Basic color correction: exposure, contrast, saturation, temperature | P1 | Applies per clip with a reset |
| FX-5 | Blend modes and opacity | P1 | Matches reference renders |
| FX-6 | OpenFX plugin hosting | P2 | A sample plugin loads on all OSes |

### 7.6 Audio (basic)

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| AUD-1 | Clip volume, fades, mute, solo | P0 | Fades render smoothly with no clicks |
| AUD-2 | Level meters and loudness readout | P1 | Matches a reference meter within 0.5 LU |
| AUD-3 | Waveform display and basic normalization | P1 | Normalizes to a target level |

### 7.7 Captions and subtitles

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| CAP-1 | Import/export SRT and VTT | P1 | Round-trip preserves timing and text |
| CAP-2 | Edit captions on the timeline | P1 | Add, trim and style captions |
| CAP-3 | Burn-in captions on export | P1 | Matches viewer rendering |

### 7.8 Export

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| EXP-1 | Export H.264/AAC in MP4 | P0 | Plays in common browsers and phones |
| EXP-2 | Export H.265 and ProRes | P1 | Uses platform encoders where available |
| EXP-3 | Export presets for common platforms | P0 | One-click presets for YouTube, Instagram, TikTok |
| EXP-4 | Render queue with progress and cancel | P1 | Multiple jobs run in order |
| EXP-5 | Export range (in/out), audio-only, image sequence | P1 | Output length matches the selection |
| EXP-6 | Hardware-accelerated encode when available | P1 | Falls back to software if unavailable |

### 7.9 Interchange

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| INT-1 | Import/export EDL | P1 | Opens in another NLE with matching cuts |
| INT-2 | Import/export OpenTimelineIO | P1 | Round-trips cuts, transitions and markers |
| INT-3 | Import FCPXML | P2 | Cuts and basic effects map correctly |

### 7.10 Application

| ID | Requirement | Priority | Acceptance criteria |
| --- | --- | --- | --- |
| APP-1 | Customizable keyboard shortcuts with presets | P1 | Shortcuts export and import |
| APP-2 | Dark and light themes | P1 | Follows the OS setting |
| APP-3 | Localization framework | P1 | UI strings are translatable; English ships in v0.1 |
| APP-4 | Accessibility: keyboard navigation, screen-reader labels, contrast | P1 | Passes an accessibility checklist |
| APP-5 | Auto-update check | P2 | Opt-in, never silent |

## 8. Non-functional requirements

**Reference machine:** 4-core CPU, 16 GB RAM, integrated GPU from 2020 or later (for example Intel Iris Xe or AMD Vega), SSD.

| Area | Requirement |
| --- | --- |
| Playback | 1080p, 3 video tracks, 30 fps, no dropped frames. 4K plays via proxies |
| Responsiveness | Timeline operations and scrubbing respond in under 100 ms |
| Startup | Under 3 seconds to an interactive window |
| Scale | A timeline of 500 clips across 8 tracks stays responsive |
| Memory | Under 4 GB for a 1080p project with 500 clips |
| Export speed | 1080p H.264 at least real-time with hardware encoding |
| Reliability | Crash recovery; no corruption on forced exit; project writes are atomic |
| Determinism | Same project exports identical frames across OSes within a defined tolerance (tested with golden frames) |
| Privacy | No telemetry by default; no network access except explicit update checks |
| Security | Media parsing runs in a restricted process where practical; dependencies scanned in CI |
| Accessibility | Keyboard-operable; screen-reader labels on controls |
| Localization | All UI text externalized |

## 9. Platform support

| Platform | Minimum | GPU path | Hardware decode/encode |
| --- | --- | --- | --- |
| Windows | 10 (64-bit) | DirectX 12 via wgpu | NVDEC/NVENC, Media Foundation, AMF, Quick Sync |
| macOS | 12 (Intel and Apple Silicon) | Metal via wgpu | VideoToolbox |
| Linux | Ubuntu 22.04 equivalent, X11 and Wayland | Vulkan via wgpu | VAAPI, NVDEC/NVENC |

Distribution: Windows installer (MSI/EXE), macOS DMG (signed and notarized), Linux AppImage and Flatpak.

## 10. Technical approach

See [ROADMAP.md](ROADMAP.md) for phases. Key constraints that shape the requirements:

- **Engine:** Rust, with a command bus so every edit is undoable and scriptable.
- **Rendering:** wgpu with a pull-based render graph and content-hash frame cache.
- **Media I/O:** FFmpeg built as **LGPL and dynamically linked**, so Thira can stay Apache-2.0. No GPL components (such as x264 and x265) in official builds.
- **UI:** Tauri with React and TypeScript.
- **Storage:** SQLite, with versioned migrations.

## 11. Formats and legal

- **Default export:** H.264/AAC via platform hardware encoders. Software fallbacks use permissively licensed encoders (OpenH264, SVT-AV1, libvpx).
- **Patents:** H.264, H.265 and AAC are patent-encumbered. Users are responsible for licensing in their jurisdiction; official builds rely on platform-provided codecs where possible. A legal review is required before the first binary release.
- **Branding:** the name, logo and UI are original. No commercial editor's names, icons or assets are used.
- **Contributions:** Developer Certificate of Origin (DCO) sign-off.

## 12. Success metrics

| Metric | v0.1 target | v1.0 target |
| --- | --- | --- |
| Test coverage of core crates | 70% | 85% |
| Open crash bugs | Under 5 | 0 critical |
| Golden-frame tests | 50 | 300 |
| Supported OS builds in CI | 3 | 3 |
| Contributors beyond the owner | 1 | 10 |
| GitHub stars (interest signal) | 100 | 1,000 |
| Time to export a 10-minute 1080p project | Under 10 minutes | Under 5 minutes |

## 13. Milestones

| Milestone | Scope | Gate |
| --- | --- | --- |
| M0 Foundations | Phase 0 | App opens, saves, reloads a project; GPU shader runs on all 3 OSes |
| M1 v0.1 (alpha) | Phase 1: all P0 requirements | 3-track 1080p timeline plays at 30 fps; H.264 export matches reference |
| M2 v0.5 (beta) | Phase 2: advanced editing, proxies, captions | 30-minute project edits smoothly with proxies |
| M3 v1.0 | Phase 3: interchange, plugin host, all P1 | Real project round-trips through another editor via OTIO or EDL |

Dates are not committed until M0 is complete and velocity is measured.

## 14. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scope grows beyond one maintainer | Project stalls | Strict P0/P1/P2 split; non-goals enforced in review |
| Playback performance misses targets | Poor first impression | Benchmarks in CI from M0; proxy-first design |
| Codec patents and licensing | Legal exposure | LGPL FFmpeg, platform codecs, legal review before release |
| GPU driver differences | Rendering bugs | wgpu, golden-frame tests on multiple GPUs |
| Cross-platform packaging complexity | Delayed releases | CI builds from M0; packaging treated as a feature |
| Low contributor interest | Slow progress | Good docs, small "good first issue" tasks, public roadmap |

## 15. Open questions

1. Final project file format: single file (zip) or folder?
2. Minimum GPU requirements for Linux (Vulkan 1.2 baseline)?
3. Whether to publish under a foundation or personal account.
4. Name and logo trademark search for "Thira".
5. Which languages to localize first after English?

## 16. Revision history

| Date | Version | Change |
| --- | --- | --- |
| 2026-11-01 | 0.1 | Initial draft |
