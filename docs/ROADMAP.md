# Thira roadmap (editing first)

Estimates assume one developer working with Claude. Each phase ends with a shippable build and a measurable gate.

| Phase | Scope | Exit gate |
| --- | --- | --- |
| 0. Foundations | Rust workspace, Tauri shell, wgpu viewer, project model, command bus with undo/redo, SQLite store, CI on Windows, macOS and Linux | App opens, saves and reloads a project, runs a GPU test shader on all three OSes |
| 1. Core editor | Media pool, FFmpeg decode, frame cache, multi-track timeline, trim tools (ripple, roll, slip, slide), snapping, playback, export (H.264, H.265, ProRes, image sequences) | 3-track 1080p timeline plays at 30 fps without drops; export matches a reference render |
| 2. Advanced editing | Transitions, titles, keyframes with easing, speed ramps and retime, nested clips, adjustment clips, multicam, subtitles, markers, proxies, render queue | A 30-minute project edits smoothly with proxies on a mid-range laptop |
| 3. Interchange and plugins | EDL, FCPXML, OTIO import and export, relinking, OpenFX host, audio basics with VST3 | A real project round-trips through another editor with no relinking errors |
| 4. Basic color | Float pipeline, OpenColorIO, primaries, curves, LUTs, scopes | 4K grade plays in real time on a mid-range GPU |
| Later | Audio mixing, compositing, AI-assisted editing, collaboration | Decided after Phase 3 |
