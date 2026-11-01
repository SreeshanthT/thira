# Architecture decision records

An ADR records one significant decision: the context, the choice, and its consequences. ADRs are numbered and never deleted. To change a decision, write a new ADR that supersedes the old one and update the old one's status.

## Process

1. Copy [template.md](template.md) to `NNNN-short-title.md` with the next number.
2. Open a pull request with status **Proposed**.
3. Set status to **Accepted** in the same pull request before it merges.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-architecture.md) | Crate boundaries, threading model and engine-to-UI IPC | Accepted |
| [0002](0002-project-file-format.md) | Project file format: a single SQLite file | Accepted |
| [0003](0003-viewer-presentation.md) | Viewer presentation: native surface under a transparent webview | Accepted |
