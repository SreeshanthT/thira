# Thira roadmap (editing first)

Estimates assume one developer working with Claude. Each phase ends with a shippable build and a measurable gate. Scope follows the priorities in [PRD.md](PRD.md); tasks for each phase are in [MILESTONES.md](MILESTONES.md).

| Phase | Milestone | Scope | Exit gate |
| --- | --- | --- | --- |
| 0. Foundations | M0 | Rust workspace, Tauri shell, wgpu viewer, project model, command bus with undo/redo, SQLite store, LGPL FFmpeg build, CI on Windows, macOS and Linux | App opens, saves and reloads a project, runs a GPU test shader on all three OSes |
| 1. Core editor | M1 (v0.1) | Media pool, FFmpeg decode, frame cache, multi-track timeline, trim tools (ripple, roll, slip, slide), snapping, markers, playback, transform, basic transitions, clip audio, autosave, H.264 export with presets | 3-track 1080p timeline plays at 30 fps without drops; export matches a reference render |
| 2. Advanced editing | M2 (v0.5) | Hardware decode, proxies, titles, keyframes with easing, speed ramps and retime, nested clips, multicam, captions, render queue, H.265 and ProRes export, range and image-sequence export | A 30-minute project edits smoothly with proxies on a mid-range laptop |
| 3. Interchange and polish | M3 (v1.0) | EDL and OTIO import and export, relinking, versioned project format, basic color correction, blend modes, audio meters and normalization, shortcuts, themes, localization, accessibility, sandboxed media parsing, documented API | A real project round-trips through another editor with no relinking errors |
| 4. Basic color | M4 | Float pipeline, OpenColorIO, primaries, curves, LUTs, scopes | 4K grade plays in real time on a mid-range GPU |
| Later | M4 | Adjustment layers, OpenFX host, FCPXML import, scripting, audio mixing, compositing, AI-assisted editing, collaboration | Decided after v1.0 |
