# Thira milestones and tasks

This breaks down [PRD.md](PRD.md) into milestones and issue-sized tasks. Each task references the PRD requirement IDs it delivers. When the PRD and [ROADMAP.md](ROADMAP.md) disagree, this plan follows the PRD priorities (see [Scope notes](#scope-notes)).

**Sizes:** S = up to 2 days, M = up to 1 week, L = 1 to 3 weeks. Split any L task before starting it.

**Task IDs** use the form `M<milestone>-<number>`. "Deps" lists tasks that must be finished first.

## Overview

| Milestone | Release | Scope | Gate | Tasks |
| --- | --- | --- | --- | --- |
| [M0 Foundations](#m0-foundations) | internal | Phase 0 | App opens, saves and reloads a project; GPU shader runs on all 3 OSes | 20 |
| [M1 Core editor](#m1-core-editor-v01-alpha) | v0.1 alpha | All P0 requirements | 3-track 1080p timeline plays at 30 fps; H.264 export matches reference | 38 |
| [M2 Advanced editing](#m2-advanced-editing-v05-beta) | v0.5 beta | Proxies, retime, keyframes, titles, captions, render queue | 30-minute project edits smoothly with proxies | 22 |
| [M3 Interchange and polish](#m3-interchange-and-polish-v10) | v1.0 | All remaining P1 | Real project round-trips through another editor via OTIO or EDL | 22 |
| [M4 After v1.0](#m4-after-v10) | v1.x | P2 and Phase 4 color | Decided after v1.0 | 10 |

Critical path: M0-05 time model → M0-07 command bus → M1-17 timeline ops → M1-10 render graph → M1-12 playback → M1-30 export → M1-37 performance gate.

---

## M0 Foundations

**Gate:** the app opens, saves and reloads a project, and runs a GPU test shader, on Windows, macOS and Linux in CI.

### Decisions (resolve PRD open questions)

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M0-01 | ADR: project file format (single zipped file vs folder) | ADR merged; choice reflected in M0-09 | Q1 | – | S |
| M0-02 | ADR: how the wgpu viewer is shown inside the Tauri window (native child surface vs frame transfer to webview) | Spike works on all 3 OSes; ADR records latency of each option | G1 | – | M |
| M0-03 | ADR: crate boundaries, threading model and engine-to-UI IPC | ADR merged with a crate diagram | G5 | – | S |
| M0-04 | Trademark search for "Thira" and Linux GPU baseline (Vulkan 1.2?) | Findings recorded in PRD open questions | Q2, Q4 | – | S |

### Engine core

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M0-05 | Rational time and timecode library: timebases, 23.976/29.97 drop-frame, frame/sample conversion | Property tests cover round-trips and drop-frame edge cases | PLY-1 | – | M |
| M0-06 | Project data model: project, timeline, track, clip, media reference, with serde | Model builds a sample timeline in tests | PRJ-1, TL-1 | M0-05 | M |
| M0-07 | Command bus: `Command` trait with apply/invert, transactions, change events | Every model mutation goes through the bus; invert tests pass | TL-6, G4 | M0-06 | M |
| M0-08 | Undo/redo history with unlimited depth and grouped transactions | Unit test: 1,000 random commands undo and redo to identical state | TL-6 | M0-07 | S |
| M0-09 | SQLite store with versioned migrations and atomic writes | Forced kill mid-save never leaves a corrupt project | PRJ-1, PRJ-5 | M0-01, M0-06 | M |
| M0-10 | Project lifecycle: new, open, save, close exposed to the UI | Reopened project is identical to the saved one (automated) | PRJ-1 | M0-09 | S |

### Rendering and media

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M0-11 | wgpu viewer surface embedded in the app window | Test pattern renders and resizes on all 3 OSes | G3 | M0-02 | L |
| M0-12 | Headless render harness and golden-frame comparison tool (with tolerance) | CI renders a test shader on software adapters (WARP, lavapipe, Metal) and compares to a golden PNG | G3 | M0-11 | M |
| M0-13 | LGPL FFmpeg build for 3 OSes, dynamically linked, with Rust bindings | Build script checks `--enable-gpl` is absent; `ffprobe`-equivalent call works in a test | MED-1 | – | L |

### App shell and infrastructure

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M0-14 | Cargo workspace, Tauri 2 shell, React + TypeScript UI with lint and format | `cargo run` opens a window on all 3 OSes | – | M0-03 | S |
| M0-15 | App layout skeleton: media pool, viewer, timeline, inspector panels; dark theme base | Panels resize and persist layout | APP-2 | M0-14 | M |
| M0-16 | UI string externalization from day one (i18n framework, English only) | Lint fails on hard-coded UI strings | APP-3 | M0-14 | S |
| M0-17 | CI matrix: build, test, clippy, UI tests on Windows, macOS, Linux; `cargo-deny` licence check (no GPL) and dependency audit | CI required on `main` | G3, G5 | M0-14 | M |
| M0-18 | Benchmark harness (criterion) running in CI with stored history | Regression over 10% fails the PR | G1 | M0-17 | S |
| M0-19 | Local logging and panic/crash log; no telemetry | Crash writes a log file; no network calls (checked in test) | Privacy | M0-14 | S |
| M0-20 | Contributor setup: CONTRIBUTING, DCO check, issue/PR templates, CODE_OF_CONDUCT, SECURITY | New contributor builds from a clean machine using the docs | G4 | – | S |

---

## M1 Core editor (v0.1 alpha)

**Gate:** all P0 requirements done; 3-track 1080p timeline plays at 30 fps with no dropped frames on the reference machine; H.264 export matches the reference render; 70% coverage of core crates; 50 golden-frame tests.

### Media

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-01 | Test media corpus (CC0/self-made) for all P0 formats, stored via Git LFS | Corpus covers MP4, MOV, MKV, WebM, MP3, WAV, PNG, JPG, VFR and odd frame rates | MED-1 | M0-13 | S |
| M1-02 | Media probing: codec, resolution, frame rate, duration, audio layout | Values match `ffprobe` for every corpus file | MED-3 | M1-01 | M |
| M1-03 | Import via dialog and drag-and-drop | All corpus files import without conversion | MED-1 | M1-02 | S |
| M1-04 | Background job system: priorities, cancellation, progress events | UI thread never blocks on a job (test) | MED-4 | M0-03 | M |
| M1-05 | Thumbnail and waveform generation with disk cache | UI stays interactive while generating for 100 clips | MED-4 | M1-04 | M |
| M1-06 | Media pool UI: bins, search, list and grid views, virtualized | 1,000 clips scroll and search without lag | MED-2 | M1-05 | M |
| M1-07 | Metadata panel in the inspector | Shows MED-3 fields for the selected clip | MED-3 | M1-02 | S |

### Decode and render

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-08 | Frame-accurate decoder with seek and decoder pool | Random seek returns the exact frame for every corpus file (frame-hash test) | PLY-1 | M0-13, M0-05 | L |
| M1-09 | GPU upload and YUV→RGB conversion (BT.601/709, full/limited range) | Golden frames match per colorspace | FX-1 | M1-08, M0-12 | M |
| M1-10 | Pull-based render graph: source, transform, composite, transition nodes | Graph renders a 3-track timeline frame | G1 | M1-09 | L |
| M1-11 | Content-hash frame cache with memory budget (LRU) | Repeat scrub over cached range does zero decodes | PLY-3 | M1-10 | M |

### Playback

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-12 | Playback engine: master clock, read-ahead, frame dropping policy | 1080p single track plays at 30 fps | PLY-1 | M1-11 | L |
| M1-13 | Audio output (cpal), track mixing, resampling | Mixed audio plays on all 3 OSes | AUD-1 | M1-08 | M |
| M1-14 | A/V sync with audio as master clock | Drift under 1 frame over 10 minutes (automated) | PLY-2 | M1-12, M1-13 | M |
| M1-15 | Transport: play, pause, frame step, J/K/L jog and shuttle | Frame step is exact; shortcuts work | PLY-1 | M1-12 | S |
| M1-16 | Scrubbing without stalls | Scrub responds in under 100 ms (benchmark in CI) | PLY-3, G1 | M1-11 | M |

### Timeline

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-17 | Edit commands: insert, overwrite, append, delete, ripple delete | Table-driven tests check clip positions for each op | TL-2 | M0-07 | M |
| M1-18 | Trim commands: ripple, roll, slip, slide | Tests cover media-limit, gap and neighbour edge cases | TL-3 | M1-17 | M |
| M1-19 | Blade at playhead and join | Split then join restores the original (property test) | TL-4 | M1-17 | S |
| M1-20 | Linked audio/video clips; track lock, mute, solo | Linked clips move and trim together; can be unlinked | TL-1, AUD-1 | M1-17 | S |
| M1-21 | Markers, in/out points, snapping engine | Snaps to clip edges, markers, playhead | TL-5 | M1-17 | M |
| M1-22 | Timeline view: 8 video + 8 audio tracks, zoom, scroll, thumbnails, waveforms; virtualized | 500 clips across 8 tracks stay under 100 ms per interaction | TL-1, G1 | M1-05, M0-15 | L |
| M1-23 | Timeline interactions: drag, drop from pool, trim handles, tool modes, keyboard editing | Each TL-2/TL-3 op is reachable by mouse and keyboard | TL-2, TL-3 | M1-22, M1-18 | L |
| M1-24 | Undo/redo UI, history panel | 1,000 UI operations undo and redo exactly (E2E test) | TL-6 | M0-08, M1-23 | S |

### Effects and audio

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-25 | Transform and crop node with inspector controls and on-viewer handles | Viewer and export match (golden frames) | FX-1 | M1-10 | M |
| M1-26 | Transitions: cross dissolve, dip to color, wipe; duration and alignment | Golden frames for each transition and alignment | FX-2 | M1-10, M1-23 | M |
| M1-27 | Clip volume and fades with click-free ramps | No clicks at fade edges (sample analysis test) | AUD-1 | M1-13 | S |

### Project

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-28 | Autosave and crash recovery | After a forced kill, reopening offers recovery within 30 s of the crash | PRJ-2, G2 | M0-09 | M |
| M1-29 | Project settings: resolution, frame rate, aspect ratio | Changing settings re-renders without data loss | PRJ-3 | M1-10 | S |

### Export

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-30 | Offline render pipeline from render graph to encoder | Exported frames match viewer frames (golden) | EXP-1, Determinism | M1-10 | M |
| M1-31 | H.264/AAC MP4 via platform encoders (Media Foundation, VideoToolbox, VAAPI) with OpenH264 fallback | Output plays in Chrome, Safari, Firefox, iOS, Android | EXP-1 | M1-30 | L |
| M1-32 | Export presets: YouTube, Instagram, TikTok | One-click export for each preset | EXP-3 | M1-31 | S |
| M1-33 | Export dialog with progress and cancel | Cancel leaves no partial file | EXP-1 | M1-31 | S |

### Release readiness

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M1-34 | Golden-frame suite to 50 tests, cross-OS tolerance defined | Suite green on all 3 OSes | Determinism | M1-25, M1-26 | M |
| M1-35 | Packaging: MSI/EXE, signed and notarized DMG, AppImage, Flatpak | Installers built by CI on tag | Platforms | M0-17 | L |
| M1-36 | Legal review of codecs and licences before the first binary release | Review recorded; NOTICE and third-party licences ship with builds | Formats and legal | M1-31 | S |
| M1-37 | Performance gate on the reference machine | Report shows 1080p 3-track 30 fps, startup under 3 s, scrub under 100 ms | G1, NFR | M1-14, M1-16 | S |
| M1-38 | User docs for every P0 feature | Docs site published from the repo | Principle 5 | – | M |

---

## M2 Advanced editing (v0.5 beta)

**Gate:** a 30-minute, 4K-source project edits smoothly with proxies on the reference machine.

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M2-01 | Hardware decode: NVDEC, VideoToolbox, VAAPI, D3D11VA/Media Foundation, with software fallback | Same frames as software decode (golden) | MED-5, Platforms | M1-08 | L |
| M2-02 | Proxy generation as background jobs | Proxies generate while editing continues | MED-5 | M1-04 | M |
| M2-03 | Automatic proxy playback and full-res export switch | 4K plays smoothly; export uses originals | MED-5 | M2-02 | M |
| M2-04 | Image sequence import | Numbered frames import as one clip | MED-6 | M1-03 | S |
| M2-05 | Speed change, reverse, freeze frame with optional pitch correction | Frame-accurate output (golden) | TL-7 | M1-10, M1-13 | L |
| M2-06 | Nested (compound) clips | Edits inside reflect on the parent timeline; undo works across levels | TL-8 | M1-17 | L |
| M2-07 | Multicam: create from clips, sync by timecode | Angles line up to the frame | TL-9 | M2-06 | M |
| M2-08 | Multicam sync by audio waveform cross-correlation | Sync within 1 frame on test pairs | TL-9 | M2-07 | M |
| M2-09 | Multicam switching in the viewer | Live switching creates cuts | TL-9 | M2-07 | M |
| M2-10 | Keyframe engine with bezier easing for position, scale, rotation, opacity, volume | Interpolation unit tests; undoable | TL-10 | M1-25 | M |
| M2-11 | Keyframe curve editor UI | Bezier handles editable in a curve view | TL-10 | M2-10 | L |
| M2-12 | Text titles: font, size, color, outline, shadow; text shaping | System fonts render identically on 3 OSes (golden) | FX-3 | M1-10 | L |
| M2-13 | SRT and VTT import/export | Round-trip preserves timing and text | CAP-1 | M1-17 | S |
| M2-14 | Caption track editing and styling | Add, trim, style captions on the timeline | CAP-2 | M2-13, M2-12 | M |
| M2-15 | Burn-in captions on export | Matches viewer rendering (golden) | CAP-3 | M2-14 | S |
| M2-16 | H.265 and ProRes export | Uses platform encoders where available | EXP-2 | M1-31 | M |
| M2-17 | Hardware-accelerated encode with software fallback | 1080p H.264 exports at least real-time with HW | EXP-6 | M1-31 | M |
| M2-18 | Render queue: multiple jobs, ordering, progress, cancel | Jobs run in order; survive app restart | EXP-4 | M1-33 | M |
| M2-19 | Export in/out range, audio-only, image sequence | Output length matches selection | EXP-5 | M1-30 | S |
| M2-20 | Full-screen and second-monitor viewer | Opens on any display | PLY-4 | M0-11 | M |
| M2-21 | Stress test: 30-minute project, 500 clips, 8 tracks | Memory under 4 GB; interactions under 100 ms | NFR Scale, Memory | M2-03 | M |
| M2-22 | User docs for M2 features | Docs updated | Principle 5 | – | S |

---

## M3 Interchange and polish (v1.0)

**Gate:** a real project round-trips through another editor via OTIO or EDL with matching cuts; all P1 done; 85% coverage of core crates; 300 golden-frame tests; zero open critical crash bugs.

| ID | Task | Done when | Refs | Deps | Size |
| --- | --- | --- | --- | --- | --- |
| M3-01 | Relink missing media for a whole moved folder | One action relinks all missing files | PRJ-4 | M1-03 | M |
| M3-02 | Document and version the project format; migration tests from every past version | Older projects open in newer builds | PRJ-5 | M0-09 | M |
| M3-03 | EDL import and export | Opens in another NLE with matching cuts | INT-1 | M1-17 | M |
| M3-04 | OpenTimelineIO import and export | Round-trips cuts, transitions, markers | INT-2 | M1-26 | L |
| M3-05 | Basic color correction: exposure, contrast, saturation, temperature, with reset | Golden frames per control | FX-4 | M1-10 | M |
| M3-06 | Blend modes and opacity | Matches reference renders | FX-5 | M1-10 | M |
| M3-07 | Audio level meters and loudness (LUFS) readout | Within 0.5 LU of a reference meter | AUD-2 | M1-13 | M |
| M3-08 | Waveform-based normalization to a target level | Normalizes to target | AUD-3 | M3-07 | S |
| M3-09 | Customizable keyboard shortcuts with presets, import/export | Shortcuts export and import | APP-1 | M1-23 | M |
| M3-10 | Light theme following the OS setting | Switches with the OS | APP-2 | M0-15 | S |
| M3-11 | Localization workflow (translation platform, pseudo-locale test) | First non-English locale ships | APP-3, Q5 | M0-16 | S |
| M3-12 | Accessibility: keyboard navigation, screen-reader labels, contrast | Passes the accessibility checklist | APP-4 | M1-23 | L |
| M3-13 | Run media parsing in a restricted helper process | Malformed file crashes the helper, not the app (fuzz test) | Security | M1-08 | L |
| M3-14 | Fuzzing of demuxer/probe inputs and project loading in CI | Fuzzers run nightly | Security, G2 | M3-13 | M |
| M3-15 | Public engine API and plugin points documented | API docs published; example script drives the command bus | G4 | M0-07 | M |
| M3-16 | Golden-frame suite to 300 tests on multiple GPU vendors | Green on NVIDIA, AMD, Intel, Apple | Determinism | M1-34 | M |
| M3-17 | Coverage of core crates to 85% | Coverage report in CI | Metrics | – | M |
| M3-18 | Export speed: 10-minute 1080p project under 5 minutes | Benchmark on reference machine | Metrics | M2-17 | M |
| M3-19 | Crash-bug burn-down to zero critical | No open critical crash issues | Metrics | – | M |
| M3-20 | Round-trip gate test with a real project | Recorded walkthrough via OTIO or EDL | Gate | M3-03, M3-04 | S |
| M3-21 | Publishing decision: foundation or personal account | Decision recorded | Q3 | – | S |
| M3-22 | v1.0 user docs, release notes, migration guide | Docs published | Principle 5 | – | M |

---

## M4 After v1.0

Not scheduled. Listed so P2 items and ROADMAP Phase 4 have a home.

| ID | Task | Refs |
| --- | --- | --- |
| M4-01 | Adjustment layers | TL-11 |
| M4-02 | Safe-area and aspect-ratio overlays | PLY-5 |
| M4-03 | OpenFX plugin hosting | FX-6 |
| M4-04 | FCPXML import | INT-3 |
| M4-05 | Opt-in auto-update check | APP-5 |
| M4-06 | Float pipeline and OpenColorIO | Phase 4 |
| M4-07 | Primaries, curves, LUTs | Phase 4 |
| M4-08 | Scopes: waveform, vectorscope, histogram | Phase 4 |
| M4-09 | Python scripting on the command bus | README stack |
| M4-10 | Audio mixing, compositing, AI-assisted editing | ROADMAP "Later" |

---

## Scope notes

Places where this plan follows the PRD over the original ROADMAP. ROADMAP.md has since been updated to match.

1. **H.265, ProRes and image-sequence export** are in ROADMAP Phase 1 but are P1 in the PRD (EXP-2, EXP-5). Placed in M2.
2. **OpenFX host** is in ROADMAP Phase 3 and the PRD's M3 says "plugin host", but FX-6 is P2. Placed in M4. M3 delivers documented API and plugin points (G4) instead.
3. **VST3** is in ROADMAP Phase 3 but audio plugin hosting is a PRD non-goal for v1.0. Left out.
4. **Adjustment clips** are in ROADMAP Phase 2 but TL-11 is P2. Placed in M4.
5. **Hardware decode** has no PRD requirement of its own but is needed for MED-5 and the Linux persona. Placed in M2.
6. **PRJ-4, PRJ-5 and APP-1 to APP-4** are P1 but not in any ROADMAP phase. Placed in M3, with groundwork (migrations, i18n) in M0.
