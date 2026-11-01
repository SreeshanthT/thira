# Contributing to Thira

Thanks for helping build Thira. This guide covers how to set up a development machine, how work is organized, and what a pull request needs before it can merge.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- Read the [PRD](docs/PRD.md) for what Thira is and is not building. Features listed as non-goals will not be merged before v1.0.
- Work is tracked as [GitHub issues](https://github.com/SreeshanthT/thira/issues), grouped into milestones. The full plan is in [MILESTONES.md](docs/MILESTONES.md).
- For anything not covered by an existing issue, open an issue first and agree on the approach before writing code.
- Significant design decisions are recorded as [ADRs](docs/adr/README.md).

## Development setup

You need the following on every platform:

| Tool | Version | Install |
| --- | --- | --- |
| Rust | latest stable | [rustup](https://rustup.rs) |
| Node.js | 22 or later | [nodejs.org](https://nodejs.org) |
| pnpm | as pinned in `package.json` | `corepack enable` (ships with Node.js) |
| Git | any recent | [git-scm.com](https://git-scm.com) |

Plus the platform prerequisites for [Tauri](https://tauri.app):

**Windows 10 or 11**

- Visual Studio 2022 Build Tools with the **Desktop development with C++** workload:
  `winget install Microsoft.VisualStudio.2022.BuildTools --override "--passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
- WebView2 runtime (preinstalled on Windows 11).

**macOS 12 or later**

- Xcode Command Line Tools: `xcode-select --install`

**Linux (Ubuntu 22.04 or equivalent)**

```sh
sudo apt install build-essential curl wget file libssl-dev libxdo-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

FFmpeg setup will be added here when media support lands ([#13](https://github.com/SreeshanthT/thira/issues/13)).

## Building and testing

The application scaffold is being added in [#14](https://github.com/SreeshanthT/thira/issues/14). Once it lands, these commands work from the repository root:

```sh
pnpm install           # install UI dependencies
pnpm tauri dev         # run the app with hot reload
cargo test --workspace # run Rust tests
pnpm test              # run UI tests
```

## Making a change

1. Pick an issue, or comment on one to say you are working on it.
2. Create a branch from `main` named `<type>/<short-description>`, for example `feat/blade-tool` or `fix/seek-off-by-one`.
3. Keep each pull request to one issue. Small pull requests are reviewed faster.
4. Add tests with the change. Engine behavior needs unit tests; rendering changes need golden-frame tests.
5. Update user docs when behavior changes (PRD principle 5).
6. Open a pull request against `main` and fill in the template. Reference the issue with `Closes #N`.

`main` is protected: every change goes through a pull request, and CI must pass on Windows, macOS and Linux before merging.

### Commit messages

- Summary line in the imperative mood, under 72 characters: "Add ripple trim", not "Added ripple trim".
- Explain *why* in the body when it is not obvious from the diff.

### Developer Certificate of Origin

Every commit must be signed off to certify that you wrote it or have the right to submit it under the project license, as described in the [Developer Certificate of Origin](https://developercertificate.org). Add the sign-off with `-s`:

```sh
git commit -s -m "Add ripple trim"
```

This appends a line like `Signed-off-by: Your Name <you@example.com>`, which must match the commit author. A CI check rejects pull requests with unsigned commits. To fix the most recent commit, run `git commit --amend -s --no-edit`; for several commits, run `git rebase --signoff main`.

## Licensing

Thira is licensed under the [Apache License 2.0](LICENSE). Contributions are accepted under the same license.

Dependencies must be compatible with Apache-2.0 distribution. GPL-licensed code and libraries cannot be added; FFmpeg is used only as an LGPL build, linked dynamically.

## Reporting bugs and security issues

- Bugs and feature ideas: [open an issue](https://github.com/SreeshanthT/thira/issues/new/choose).
- Security vulnerabilities: do **not** open a public issue. Follow [SECURITY.md](SECURITY.md).
