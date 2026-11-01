# Thira

**Thira** (Malayalam: *wave*, and the cinema screen) is an open-source, cross-platform video editor for **Windows, macOS and Linux**.

> Status: **planning / Phase 0**. Nothing is usable yet.

## Goals

- A fast, non-destructive, professional **editing** experience first: timeline, trimming, multicam, transitions, titles, keyframes, proxies and export.
- One shared project model and one GPU render pipeline, so later additions (color, audio mixing, compositing) plug in without rewrites.
- Honest, documented behavior: every feature has tests, and every phase has a measurable performance gate.

## Planned stack

| Area | Choice |
| --- | --- |
| Engine | Rust |
| GPU | wgpu (DirectX 12 / Vulkan / Metal) |
| Media I/O | FFmpeg |
| UI | Tauri + React + TypeScript |
| Project storage | SQLite |
| Scripting | Python (later) |

## Roadmap

See [docs/PRD.md](docs/PRD.md) for requirements and [docs/ROADMAP.md](docs/ROADMAP.md) for phases. The first milestone is a usable editor (Phases 0 to 2).

## Contributing

The project is at its very start. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, workflow and the DCO sign-off, and open an issue to discuss ideas before sending a pull request. The `main` branch is protected: all changes go through pull requests.

## License

Licensed under the [Apache License 2.0](LICENSE).
